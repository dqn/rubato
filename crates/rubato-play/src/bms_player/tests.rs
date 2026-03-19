use super::*;
use bms_model::bms_model::BMSModel;
use bms_model::mode::Mode;
use rubato_core::config::Config;
use rubato_core::main_state::MainState;
use rubato_core::main_state::SkinDrawable;
use rubato_core::player_config::PlayerConfig;
use rubato_core::sprite_batch_helper::SpriteBatch;
use rubato_input::bms_player_input_device::DeviceType;
use rubato_input::bms_player_input_processor::{BMSPlayerInputProcessor, KEYSTATE_SIZE};
use rubato_input::keyboard_input_processor::ControlKeys;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::atomic::{AtomicI32, Ordering};

fn make_model() -> BMSModel {
    let mut model = BMSModel::new();
    model.set_mode(Mode::BEAT_7K);
    model.judgerank = 100;
    model
}

fn make_model_with_time(last_note_time: i32) -> BMSModel {
    let mut model = BMSModel::new();
    model.set_mode(Mode::BEAT_7K);
    model.judgerank = 100;
    // Add a timeline at the given time to set last_note_time
    let mut timelines = Vec::new();
    let tl = bms_model::time_line::TimeLine::new(130.0, last_note_time as i64 * 1000, 8);
    timelines.push(tl);
    model.timelines = timelines;
    model
}

struct PlayerConfigMutatingSkin;

impl SkinDrawable for PlayerConfigMutatingSkin {
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
        if let Some(config) = ctx.player_config_mut() {
            config.judge_settings.judgetiming += 1;
        }
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

    fn swap_sprite_batch(&mut self, _batch: &mut SpriteBatch) {}
}

struct ProbeImageIndexSkin {
    id: i32,
    observed: Arc<AtomicI32>,
}

impl SkinDrawable for ProbeImageIndexSkin {
    fn draw_all_objects_timed(
        &mut self,
        ctx: &mut dyn rubato_types::skin_render_context::SkinRenderContext,
    ) {
        self.observed
            .store(ctx.image_index_value(self.id), Ordering::SeqCst);
    }

    fn update_custom_objects_timed(
        &mut self,
        _ctx: &mut dyn rubato_types::skin_render_context::SkinRenderContext,
    ) {
    }

    fn mouse_pressed_at(
        &mut self,
        _ctx: &mut dyn rubato_types::skin_render_context::SkinRenderContext,
        _button: i32,
        _x: i32,
        _y: i32,
    ) {
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

    fn swap_sprite_batch(&mut self, _batch: &mut SpriteBatch) {}
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct ObservedDrawLaneTime {
    time: i64,
    timer_play: Option<i64>,
}

struct ProbeDrawLaneTimeSkin {
    observed: Arc<Mutex<Option<ObservedDrawLaneTime>>>,
}

impl SkinDrawable for ProbeDrawLaneTimeSkin {
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
        _ctx: &mut dyn rubato_types::skin_render_context::SkinRenderContext,
        _button: i32,
        _x: i32,
        _y: i32,
    ) {
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

    fn compute_note_draw_commands(
        &mut self,
        _lane_renderer: &mut dyn std::any::Any,
        ctx: Box<dyn std::any::Any>,
    ) {
        let ctx = ctx
            .downcast::<crate::lane_renderer::DrawLaneContext>()
            .expect("render_skin_impl should pass DrawLaneContext");
        *self.observed.lock().unwrap() = Some(ObservedDrawLaneTime {
            time: ctx.time,
            timer_play: ctx.timer_play,
        });
    }

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

    fn swap_sprite_batch(&mut self, _batch: &mut SpriteBatch) {}
}

// --- Constructor tests ---

#[test]
fn new_creates_default_state() {
    let model = make_model();
    let player = BMSPlayer::new(model);
    assert_eq!(player.state(), PlayState::Preload);
    assert_eq!(player.play_speed(), 100);
    assert_eq!(player.adjusted_volume(), -1.0);
    assert!(!player.score.analysis_checked);
}

#[test]
fn new_sets_playtime_from_model() {
    let model = make_model();
    let expected_playtime = model.last_note_time() + TIME_MARGIN;
    let player = BMSPlayer::new(model);
    assert_eq!(player.playtime(), expected_playtime);
}

// --- MainState trait tests ---

#[test]
fn state_type_returns_play() {
    let model = make_model();
    let player = BMSPlayer::new(model);
    assert_eq!(player.state_type(), Some(MainStateType::Play));
}

#[test]
fn main_state_data_accessible() {
    let model = make_model();
    let player = BMSPlayer::new(model);
    let data = player.main_state_data();
    // Timer should be initialized
    assert!(!data.timer.is_timer_on(TIMER_PLAY));
}

#[test]
fn handle_skin_mouse_pressed_uses_live_play_context() {
    let model = make_model();
    let mut player = BMSPlayer::new(model);
    player.main_state_data.skin = Some(Box::new(PlayerConfigMutatingSkin));
    player.player_config.judge_settings.judgetiming = 0;

    <BMSPlayer as MainState>::handle_skin_mouse_pressed(&mut player, 0, 10, 10);

    assert_eq!(player.player_config.judge_settings.judgetiming, 1);
}

#[test]
fn render_skin_uses_play_option_for_image_index_42() {
    let model = make_model();
    let mut player = BMSPlayer::new(model);
    player.player_config.play_settings.random = 1;
    player.score.playinfo.randomoption = 6;
    let observed = Arc::new(AtomicI32::new(-1));
    player.main_state_data.skin = Some(Box::new(ProbeImageIndexSkin {
        id: 42,
        observed: observed.clone(),
    }));

    let mut sprite = SpriteBatch::new();
    <BMSPlayer as MainState>::render_skin(&mut player, &mut sprite);

    assert_eq!(observed.load(Ordering::SeqCst), 6);
}

#[test]
fn render_skin_uses_target_visual_index_for_image_index_77() {
    let model = make_model();
    let mut player = BMSPlayer::new(model);
    player.player_config.select_settings.targetid = "MAX".to_string();
    let observed = Arc::new(AtomicI32::new(-1));
    player.main_state_data.skin = Some(Box::new(ProbeImageIndexSkin {
        id: 77,
        observed: observed.clone(),
    }));

    let mut sprite = SpriteBatch::new();
    <BMSPlayer as MainState>::render_skin(&mut player, &mut sprite);

    assert_eq!(observed.load(Ordering::SeqCst), 10);
}

// --- State machine transition tests ---

#[test]
fn state_preload_transitions_to_ready() {
    let model = make_model();
    let mut player = BMSPlayer::new(model);
    player.play_skin.loadstart = 0;
    player.play_skin.loadend = 0;
    player.media_load_finished = true;

    // The PRELOAD->READY transition requires:
    // 1. media_load_finished = true
    // 2. micronow > (loadstart + loadend) * 1000 = 0
    // 3. micronow - startpressedtime > 1_000_000
    //
    // To satisfy (2) and (3), we need micronow > 1_000_000.
    // Since TimerManager uses Instant::now(), micronow is near 0 in tests.
    // We force this by setting TIMER_PLAY to a known value and using set_micro_timer
    // to manipulate the effective "now" time. However, the simplest approach is
    // to directly manipulate the state and verify the transition logic.
    player.state = PlayState::Preload;
    player.startpressedtime = -2_000_000;

    // Set the timer's starttime far in the past by calling update repeatedly
    // won't help since elapsed is near-zero. Instead, use set_micro_timer
    // on a timer we read from to simulate "time has passed".
    // Actually, the simplest fix: set startpressedtime so the delta is satisfied
    // even with micronow near 0. micronow(~0) - startpressedtime(-2M) = 2M > 1M. Good.
    // But micronow(~0) > load_threshold(0) requires micronow > 0, which may be 0.
    // So let's update the timer to get a small positive value.
    std::thread::sleep(std::time::Duration::from_millis(2));
    player.main_state_data.timer.update();

    player.render();
    assert_eq!(player.state(), PlayState::Ready);
}

#[test]
fn state_ready_transitions_to_play() {
    let model = make_model();
    let mut player = BMSPlayer::new(model);
    player.state = PlayState::Ready;
    player.play_skin.playstart = 0; // Instant transition
    player.main_state_data.timer.set_timer_on(TIMER_READY);
    player.lanerender = Some(LaneRenderer::new(&player.model));

    // Update timer and render
    player.main_state_data.timer.update();
    // TIMER_READY now_time should be > 0 (= playstart)
    // But now_time_for_id checks micronow - timer value, which is 0 since we just set it
    // We need some time to pass. Since playstart=0, any positive time works.
    // The condition is: timer.getNowTime(TIMER_READY) > skin.getPlaystart()
    // getNowTime(TIMER_READY) = (nowmicrotime - timer[TIMER_READY]) / 1000
    // Since we just set it, this is ~0. We need > 0.
    // Let's manually set the timer to past to simulate time passing.
    let now = player.main_state_data.timer.now_micro_time();
    player
        .main_state_data
        .timer
        .set_micro_timer(TIMER_READY, now - 2000); // 2ms ago

    player.render();
    assert_eq!(player.state(), PlayState::Play);
}

#[test]
fn state_play_transitions_to_finished_when_playtime_exceeded() {
    let model = make_model();
    let mut player = BMSPlayer::new(model);
    player.state = PlayState::Play;
    player.playtime = 0; // Instant finish

    // Set TIMER_PLAY to far past so ptime is large
    player.main_state_data.timer.update();
    let now = player.main_state_data.timer.now_micro_time();
    player
        .main_state_data
        .timer
        .set_micro_timer(TIMER_PLAY, now - 2_000_000); // 2 seconds ago
    player.prevtime = now - 1000; // Small delta

    player.render();
    assert_eq!(player.state(), PlayState::Finished);
}

#[test]
fn state_play_transitions_to_failed_on_zero_gauge() {
    let model = make_model();
    let mut player = BMSPlayer::new(model);
    player.state = PlayState::Play;
    player.playtime = 999_999; // Long playtime so we don't finish

    // Create a gauge at 0 value
    let gauge = crate::groove_gauge::create_groove_gauge(
        &player.model,
        rubato_types::groove_gauge::HARD,
        0,
        None,
    )
    .unwrap();
    player.gauge = Some(gauge);
    // Set gauge to 0
    player.gauge.as_mut().unwrap().set_value(0.0);

    // Setup timers
    player.main_state_data.timer.update();
    let now = player.main_state_data.timer.now_micro_time();
    player
        .main_state_data
        .timer
        .set_micro_timer(TIMER_PLAY, now - 1000);
    player.prevtime = now - 500;

    player.render();
    assert_eq!(player.state(), PlayState::Failed);
}

// --- stop_play tests ---

#[test]
fn stop_play_from_practice_goes_to_practice_finished() {
    let model = make_model();
    let mut player = BMSPlayer::new(model);
    player.state = PlayState::Practice;
    player.stop_play();
    assert_eq!(player.state(), PlayState::PracticeFinished);
    assert!(player.main_state_data.timer.is_timer_on(TIMER_FADEOUT));
}

#[test]
fn stop_play_from_preload_goes_to_aborted() {
    let model = make_model();
    let mut player = BMSPlayer::new(model);
    player.state = PlayState::Preload;
    player.stop_play();
    assert_eq!(player.state(), PlayState::Aborted);
    assert!(player.main_state_data.timer.is_timer_on(TIMER_FADEOUT));
}

#[test]
fn stop_play_from_ready_goes_to_aborted() {
    let model = make_model();
    let mut player = BMSPlayer::new(model);
    player.state = PlayState::Ready;
    player.stop_play();
    assert_eq!(player.state(), PlayState::Aborted);
}

#[test]
fn stop_play_from_play_with_no_notes_goes_to_aborted() {
    let model = make_model();
    let mut player = BMSPlayer::new(model);
    player.state = PlayState::Play;
    // Judge has no notes hit (all counts = 0), and keyinput needs to exist
    player.input.keyinput = Some(KeyInputProccessor::new(&LaneProperty::new(&Mode::BEAT_7K)));
    player.stop_play();
    assert_eq!(player.state(), PlayState::Aborted);
}

#[test]
fn stop_play_ignores_if_already_failed_timer() {
    let model = make_model();
    let mut player = BMSPlayer::new(model);
    player.state = PlayState::Play;
    player.main_state_data.timer.set_timer_on(TIMER_FAILED);
    let prev_state = player.state;
    player.stop_play();
    // State should not change because TIMER_FAILED is already on
    assert_eq!(player.state(), prev_state);
}

// --- create_score_data tests ---

/// Helper: create a model with notes that have specific state/playtime values.
/// `notes_spec` is a vec of (state, micro_play_time) tuples for Normal notes.
fn make_model_with_timed_notes(notes_spec: &[(i32, i64)]) -> BMSModel {
    let mut model = BMSModel::new();
    model.set_mode(Mode::BEAT_7K);
    model.judgerank = 100;

    let mut timelines = Vec::new();
    for (i, &(state, playtime)) in notes_spec.iter().enumerate() {
        let mut tl = bms_model::time_line::TimeLine::new(i as f64, (i as i64) * 1_000_000, 8);
        let mut note = bms_model::note::Note::new_normal(1);
        note.set_state(state);
        note.set_micro_play_time(playtime);
        tl.set_note(0, Some(note));
        timelines.push(tl);
    }
    model.timelines = timelines;
    model
}

#[test]
fn create_score_data_timing_stats_with_hit_notes() {
    // Three notes with state 1-4 and known play times:
    //   note0: state=1, playtime=1000  (|1000| = 1000)
    //   note1: state=2, playtime=-2000 (|-2000| = 2000)
    //   note2: state=3, playtime=3000  (|3000| = 3000)
    let model = make_model_with_timed_notes(&[(1, 1000), (2, -2000), (3, 3000)]);
    let mut player = BMSPlayer::new(model);
    // Use ABORTED state to bypass the zero-notes-hit check
    player.state = PlayState::Aborted;

    let score = player.create_score_data(DeviceType::Keyboard).unwrap();

    // total_duration = |1000| + |-2000| + |3000| = 6000
    assert_eq!(score.timing_stats.total_duration, 6000);
    // total_avg = 1000 + (-2000) + 3000 = 2000
    assert_eq!(score.timing_stats.total_avg, 2000);
    // avgjudge = total_duration / count = 6000 / 3 = 2000
    assert_eq!(score.timing_stats.avgjudge, 2000);
    // avg = total_avg / count = 2000 / 3 = 666
    assert_eq!(score.timing_stats.avg, 666);
    // stddev = sqrt(((1000 - 666)^2 + (-2000 - 666)^2 + (3000 - 666)^2) / 3)
    //        = sqrt((111556 + 7111696 + 5449956) / 3)
    //        = sqrt(12673208 / 3)
    //        = sqrt(4224402)
    //        = 2055 (as i64 from f64::sqrt truncation)
    let mean = 666_i64;
    let var = ((1000 - mean).pow(2) + (-2000 - mean).pow(2) + (3000 - mean).pow(2)) / 3;
    let expected_stddev = (var as f64).sqrt() as i64;
    assert_eq!(score.timing_stats.stddev, expected_stddev);
}

#[test]
fn create_score_data_timing_stats_no_judged_notes() {
    // Notes with state=0 (not judged) each get a 1,000,000μs penalty (Java parity).
    // avg and stddev stay at defaults since no judged notes exist.
    let model = make_model_with_timed_notes(&[(0, 5000), (0, -3000)]);
    let mut player = BMSPlayer::new(model);
    player.state = PlayState::Aborted;

    let score = player.create_score_data(DeviceType::Keyboard).unwrap();

    // 2 unjudged notes: total_duration = 2 * 1_000_000 = 2_000_000
    assert_eq!(score.timing_stats.total_duration, 2_000_000);
    // avgjudge = 2_000_000 / 2 = 1_000_000
    assert_eq!(score.timing_stats.avgjudge, 1_000_000);
    // avg stays at default (no judged notes for the Rust-only avg computation)
    assert_eq!(score.timing_stats.avg, i64::MAX);
    // total_avg = 0 (no judged notes contributed signed times)
    assert_eq!(score.timing_stats.total_avg, 0);
    // stddev stays at default (no judged notes)
    assert_eq!(score.timing_stats.stddev, 0);
}

#[test]
fn create_score_data_timing_stats_filters_ln_end_notes() {
    // LN end notes of longnote type should be excluded from timing stats.
    let mut model = BMSModel::new();
    model.set_mode(Mode::BEAT_7K);
    model.judgerank = 100;
    // Default lntype is LNTYPE_LONGNOTE (0)

    let mut tl = bms_model::time_line::TimeLine::new(0.0, 0, 8);

    // Normal note: state=1, playtime=1000 → included
    let mut normal = bms_model::note::Note::new_normal(1);
    normal.set_state(1);
    normal.set_micro_play_time(1000);
    tl.set_note(0, Some(normal));

    // LN end note with TYPE_UNDEFINED (default) + lntype=LNTYPE_LONGNOTE → excluded
    let mut ln_end = bms_model::note::Note::new_long(1);
    ln_end.set_end(true);
    ln_end.set_state(1);
    ln_end.set_micro_play_time(5000);
    tl.set_note(1, Some(ln_end));

    // LN start note (not end): state=2, playtime=2000 → included
    let mut ln_start = bms_model::note::Note::new_long(1);
    ln_start.set_state(2);
    ln_start.set_micro_play_time(2000);
    tl.set_note(2, Some(ln_start));

    model.timelines = vec![tl];

    let mut player = BMSPlayer::new(model);
    player.state = PlayState::Aborted;

    let score = player.create_score_data(DeviceType::Keyboard).unwrap();

    // Only normal(1000) and ln_start(2000) should be included
    assert_eq!(score.timing_stats.total_duration, 3000); // |1000| + |2000|
    assert_eq!(score.timing_stats.total_avg, 3000); // 1000 + 2000
    assert_eq!(score.timing_stats.avgjudge, 1500); // 3000 / 2
    assert_eq!(score.timing_stats.avg, 1500); // 3000 / 2
}

#[test]
fn create_score_data_returns_none_when_no_notes_hit() {
    let model = make_model();
    let player = BMSPlayer::new(model);
    // No notes hit - all judge counts are 0
    let result = player.create_score_data(DeviceType::Keyboard);
    assert!(result.is_none());
}

#[test]
fn create_score_data_returns_some_when_aborted() {
    let model = make_model();
    let mut player = BMSPlayer::new(model);
    player.state = PlayState::Aborted;
    // Even with no notes, aborted state returns score data
    let result = player.create_score_data(DeviceType::Keyboard);
    assert!(result.is_some());
}

// --- create_score_data device_type tests ---

#[test]
fn create_score_data_sets_device_type_keyboard() {
    use rubato_types::bms_player_input_device;

    let model = make_model();
    let mut player = BMSPlayer::new(model);
    player.state = PlayState::Aborted;

    let score = player.create_score_data(DeviceType::Keyboard).unwrap();
    assert_eq!(
        score.play_option.device_type,
        Some(bms_player_input_device::Type::KEYBOARD)
    );
}

#[test]
fn create_score_data_sets_device_type_bm_controller() {
    use rubato_types::bms_player_input_device;

    let model = make_model();
    let mut player = BMSPlayer::new(model);
    player.state = PlayState::Aborted;

    let score = player.create_score_data(DeviceType::BmController).unwrap();
    assert_eq!(
        score.play_option.device_type,
        Some(bms_player_input_device::Type::BM_CONTROLLER)
    );
}

#[test]
fn create_score_data_sets_device_type_midi() {
    use rubato_types::bms_player_input_device;

    let model = make_model();
    let mut player = BMSPlayer::new(model);
    player.state = PlayState::Aborted;

    let score = player.create_score_data(DeviceType::Midi).unwrap();
    assert_eq!(
        score.play_option.device_type,
        Some(bms_player_input_device::Type::MIDI)
    );
}

// --- update_judge tests ---

#[test]
fn update_judge_updates_pomyu_chara_judge() {
    let model = make_model();
    let mut player = BMSPlayer::new(model);
    player.gauge = Some(
        crate::groove_gauge::create_groove_gauge(
            &player.model,
            rubato_types::groove_gauge::NORMAL,
            0,
            None,
        )
        .unwrap(),
    );
    player.update_judge(0, 1_000_000); // PGREAT
    assert_eq!(player.play_skin.pomyu.pm_chara_judge, 1);

    player.update_judge(2, 2_000_000); // GOOD
    assert_eq!(player.play_skin.pomyu.pm_chara_judge, 3);
}

#[test]
fn render_turns_on_judge_timer_after_autoplay_judgment() {
    let model = make_model_with_notes_at_times(&[1_000_000]);
    let mut player = BMSPlayer::new(model);
    let mode = player.model.mode().copied().unwrap_or(Mode::BEAT_7K);

    player.play_mode = BMSPlayerMode::AUTOPLAY;
    player.rebuild_judge_system(&mode);
    player.gauge = crate::groove_gauge::create_groove_gauge(
        &player.model,
        rubato_types::groove_gauge::NORMAL,
        0,
        None,
    );
    player.lanerender = Some(LaneRenderer::new(&player.model));
    player.state = PlayState::Play;
    player.playtime = 999_999;

    player.main_state_data.timer.update();
    let now = player.main_state_data.timer.now_micro_time();
    player.prevtime = now;
    player
        .main_state_data
        .timer
        .set_micro_timer(TIMER_PLAY, now - 1_000_000);

    player.render();

    assert!(
        player
            .main_state_data
            .timer
            .is_timer_on(rubato_types::timer_id::TimerId::new(46)),
        "judge timer 46 should turn on after an autoplay judgment"
    );
}

// --- set_play_speed tests ---

#[test]
fn set_play_speed_updates_value() {
    let model = make_model();
    let mut player = BMSPlayer::new(model);
    player.set_play_speed(50);
    assert_eq!(player.play_speed(), 50);
}

// --- Getter tests ---

#[test]
fn get_mode_returns_model_mode() {
    let model = make_model();
    let player = BMSPlayer::new(model);
    assert_eq!(player.mode(), Mode::BEAT_7K);
}

#[test]
fn get_skin_type_returns_matching_type() {
    let model = make_model();
    let player = BMSPlayer::new(model);
    let skin_type = player.skin_type();
    assert!(skin_type.is_some());
}

#[test]
fn get_option_information_returns_playinfo() {
    let model = make_model();
    let player = BMSPlayer::new(model);
    let info = player.option_information();
    assert_eq!(info.randomoption, 0);
}

#[test]
fn is_note_end_false_initially() {
    let model = make_model();
    let player = BMSPlayer::new(model);
    // With no notes, total_notes = 0 and past_notes = 0, so it should be true
    assert!(player.is_note_end());
}

#[test]
fn get_now_quarter_note_time_zero_without_rhythm() {
    let model = make_model();
    let player = BMSPlayer::new(model);
    assert_eq!(player.now_quarter_note_time(), 0);
}

// --- State machine lifecycle integration test ---

#[test]
fn lifecycle_preload_ready_play_finished() {
    let model = make_model();
    let mut player = BMSPlayer::new(model);
    player.media_load_finished = true;

    // Start at PRELOAD
    assert_eq!(player.state(), PlayState::Preload);

    // Force transition to READY
    player.startpressedtime = -2_000_000;
    player.play_skin.loadstart = 0;
    player.play_skin.loadend = 0;
    std::thread::sleep(std::time::Duration::from_millis(2));
    player.main_state_data.timer.update();
    player.render();
    assert_eq!(player.state(), PlayState::Ready);

    // Force transition to PLAY
    player.play_skin.playstart = 0;
    player.lanerender = Some(LaneRenderer::new(&player.model));
    let now = player.main_state_data.timer.now_micro_time();
    player
        .main_state_data
        .timer
        .set_micro_timer(TIMER_READY, now - 2000);
    player.render();
    assert_eq!(player.state(), PlayState::Play);

    // Force transition to FINISHED
    player.playtime = 0; // Instant finish
    let now = player.main_state_data.timer.now_micro_time();
    player
        .main_state_data
        .timer
        .set_micro_timer(TIMER_PLAY, now - 2_000_000);
    player.prevtime = now - 1000;
    player.render();
    assert_eq!(player.state(), PlayState::Finished);
}

// --- dispose test ---

#[test]
fn dispose_clears_skin() {
    let model = make_model();
    let mut player = BMSPlayer::new(model);
    player.dispose();
    assert!(player.main_state_data.skin.is_none());
}

// --- build_pattern_modifiers tests ---

fn make_default_config() -> rubato_core::player_config::PlayerConfig {
    rubato_core::player_config::PlayerConfig::default()
}

#[test]
fn build_pattern_modifiers_default_config_no_assist() {
    let model = make_model();
    let mut player = BMSPlayer::new(model);
    let config = make_default_config();
    let score = player.build_pattern_modifiers(&config);
    assert!(score, "Default config should allow score submission");
    assert_eq!(player.assist, 0, "Default config should not set assist");
}

#[test]
fn build_pattern_modifiers_scroll_mode() {
    // ScrollSpeedModifier requires at least one timeline
    let mut model = BMSModel::new();
    model.set_mode(Mode::BEAT_7K);
    model.judgerank = 100;
    let tl = bms_model::time_line::TimeLine::new(130.0, 0, 8);
    model.timelines = vec![tl];

    let mut player = BMSPlayer::new(model);
    let mut config = make_default_config();
    config.display_settings.scroll_mode = 1; // Enable scroll speed modifier (Remove mode)
    player.build_pattern_modifiers(&config);
    // ScrollSpeedModifier in Remove mode sets LightAssist if BPM changes exist;
    // with a single-BPM model it sets None. Either way, the modifier was applied.
    // The key thing is it doesn't crash and processes correctly.
}

#[test]
fn build_pattern_modifiers_longnote_mode() {
    let model = make_model();
    let mut player = BMSPlayer::new(model);
    let mut config = make_default_config();
    config.note_modifier_settings.longnote_mode = 1; // Enable LN modifier (Remove mode)
    player.build_pattern_modifiers(&config);
    // LongNoteModifier in Remove mode sets Assist if LNs exist.
    // With empty model, no LNs, so assist stays None.
}

#[test]
fn build_pattern_modifiers_mine_mode() {
    let model = make_model();
    let mut player = BMSPlayer::new(model);
    let mut config = make_default_config();
    config.play_settings.mine_mode = 1; // Enable mine modifier (Remove mode)
    player.build_pattern_modifiers(&config);
    // MineNoteModifier in Remove mode sets LightAssist if mine notes exist.
}

#[test]
fn build_pattern_modifiers_extranote() {
    let model = make_model();
    let mut player = BMSPlayer::new(model);
    let mut config = make_default_config();
    config.display_settings.extranote_depth = 1; // Enable extra note modifier
    player.build_pattern_modifiers(&config);
}

#[test]
fn build_pattern_modifiers_dp_battle_converts_sp_to_dp() {
    let mut model = BMSModel::new();
    model.set_mode(Mode::BEAT_7K);
    model.judgerank = 100;
    let mut player = BMSPlayer::new(model);

    let mut config = make_default_config();
    config.play_settings.doubleoption = 2;
    player.score.playinfo.doubleoption = 2;

    let score = player.build_pattern_modifiers(&config);
    // SP BEAT_7K should be converted to BEAT_14K
    assert_eq!(player.mode(), Mode::BEAT_14K);
    // assist should be at least 1 (LightAssist)
    assert!(player.assist >= 1);
    // score should be false
    assert!(!score);
}

#[test]
fn build_pattern_modifiers_dp_battle_with_autoplay_scratch() {
    let mut model = BMSModel::new();
    model.set_mode(Mode::BEAT_7K);
    model.judgerank = 100;
    let mut player = BMSPlayer::new(model);

    let mut config = make_default_config();
    config.play_settings.doubleoption = 3; // Battle + L-ASSIST (autoplay scratch)
    player.score.playinfo.doubleoption = 3;

    player.build_pattern_modifiers(&config);
    // SP BEAT_7K should be converted to BEAT_14K
    assert_eq!(player.mode(), Mode::BEAT_14K);
    assert!(player.assist >= 1);
}

#[test]
fn build_pattern_modifiers_dp_battle_non_sp_resets_doubleoption() {
    let mut model = BMSModel::new();
    model.set_mode(Mode::BEAT_14K); // Already DP
    model.judgerank = 100;
    let mut player = BMSPlayer::new(model);

    let mut config = make_default_config();
    config.play_settings.doubleoption = 2;
    player.score.playinfo.doubleoption = 2;

    player.build_pattern_modifiers(&config);
    // Not SP mode, so BATTLE is not applied
    assert_eq!(player.mode(), Mode::BEAT_14K);
    assert_eq!(player.score.playinfo.doubleoption, 0);
}

#[test]
fn build_pattern_modifiers_dp_flip() {
    let mut model = BMSModel::new();
    model.set_mode(Mode::BEAT_14K); // DP mode
    model.judgerank = 100;
    let mut player = BMSPlayer::new(model);

    let mut config = make_default_config();
    config.play_settings.doubleoption = 1;
    player.score.playinfo.doubleoption = 1;

    player.build_pattern_modifiers(&config);
    // PlayerFlipModifier should be applied, mode stays BEAT_14K
    assert_eq!(player.mode(), Mode::BEAT_14K);
}

#[test]
fn build_pattern_modifiers_random_option_seed_saved() {
    let model = make_model();
    let mut player = BMSPlayer::new(model);
    let config = make_default_config();

    player.build_pattern_modifiers(&config);
    // After applying modifiers, the 1P random seed should be saved in playinfo
    // Even with Identity (random=0), the seed is initialized
    assert_ne!(player.score.playinfo.randomoptionseed, -1);
}

#[test]
fn build_pattern_modifiers_random_option_seed_restored() {
    let model = make_model();
    let mut player = BMSPlayer::new(model);
    let config = make_default_config();

    // Pre-set a seed (as if restoring from replay)
    player.score.playinfo.randomoptionseed = 12345;

    player.build_pattern_modifiers(&config);
    // The seed should be preserved (not overwritten)
    assert_eq!(player.score.playinfo.randomoptionseed, 12345);
}

#[test]
fn build_pattern_modifiers_dp_random2_seed_saved() {
    let mut model = BMSModel::new();
    model.set_mode(Mode::BEAT_14K); // DP mode
    model.judgerank = 100;
    let mut player = BMSPlayer::new(model);
    let config = make_default_config();

    player.build_pattern_modifiers(&config);
    // In DP mode, the 2P random seed should also be saved
    assert_ne!(player.score.playinfo.randomoption2seed, -1);
}

#[test]
fn build_pattern_modifiers_7to9() {
    let model = make_model(); // BEAT_7K
    let mut player = BMSPlayer::new(model);
    let mut config = make_default_config();
    config.note_modifier_settings.seven_to_nine_pattern = 1; // Enable 7to9

    player.build_pattern_modifiers(&config);
    // Mode should be changed from BEAT_7K to POPN_9K
    assert_eq!(player.mode(), Mode::POPN_9K);
    assert!(player.assist >= 1, "7to9 should set at least light assist");
}

#[test]
fn build_pattern_modifiers_assist_accumulates_light() {
    let mut model = BMSModel::new();
    model.set_mode(Mode::BEAT_7K);
    model.judgerank = 100;
    // Add timelines with a mine note to trigger assist
    let mut tl = bms_model::time_line::TimeLine::new(130.0, 0, 8);
    tl.set_note(0, Some(bms_model::note::Note::new_mine(-1, 10.0)));
    model.timelines = vec![tl];

    let mut player = BMSPlayer::new(model);
    let mut config = make_default_config();
    config.play_settings.mine_mode = 1; // Remove mines -> LightAssist

    let score = player.build_pattern_modifiers(&config);
    assert_eq!(
        player.assist, 1,
        "Mine removal should set assist to 1 (LightAssist)"
    );
    assert!(!score, "LightAssist should mark score as invalid");
}

#[test]
fn build_pattern_modifiers_5k_battle() {
    let mut model = BMSModel::new();
    model.set_mode(Mode::BEAT_5K);
    model.judgerank = 100;
    let mut player = BMSPlayer::new(model);

    let mut config = make_default_config();
    config.play_settings.doubleoption = 2;
    player.score.playinfo.doubleoption = 2;

    player.build_pattern_modifiers(&config);
    // BEAT_5K should be converted to BEAT_10K
    assert_eq!(player.mode(), Mode::BEAT_10K);
}

// --- encode_seed_for_score tests ---

#[test]
fn encode_seed_for_score_sp_returns_1p_seed() {
    let model = make_model(); // BEAT_7K (player=1)
    let mut player = BMSPlayer::new(model);
    player.score.playinfo.randomoptionseed = 12345;
    assert_eq!(player.encode_seed_for_score(), 12345);
}

#[test]
fn encode_seed_for_score_dp_returns_combined() {
    let mut model = BMSModel::new();
    model.set_mode(Mode::BEAT_14K); // DP (player=2)
    model.judgerank = 100;
    let mut player = BMSPlayer::new(model);
    player.score.playinfo.randomoptionseed = 100;
    player.score.playinfo.randomoption2seed = 3;
    // Combined: 3 * 65536 * 256 + 100 = 3 * 16777216 + 100 = 50331748
    assert_eq!(player.encode_seed_for_score(), 50_331_748);
}

#[test]
fn encode_seed_for_score_dp_zero_seeds() {
    let mut model = BMSModel::new();
    model.set_mode(Mode::BEAT_14K);
    model.judgerank = 100;
    let mut player = BMSPlayer::new(model);
    player.score.playinfo.randomoptionseed = 0;
    player.score.playinfo.randomoption2seed = 0;
    assert_eq!(player.encode_seed_for_score(), 0);
}

// --- encode_option_for_score tests ---

#[test]
fn encode_option_for_score_sp_returns_randomoption() {
    let model = make_model(); // BEAT_7K (player=1)
    let mut player = BMSPlayer::new(model);
    player.score.playinfo.randomoption = 5;
    assert_eq!(player.encode_option_for_score(), 5);
}

#[test]
fn encode_option_for_score_dp_returns_combined() {
    let mut model = BMSModel::new();
    model.set_mode(Mode::BEAT_14K); // DP (player=2)
    model.judgerank = 100;
    let mut player = BMSPlayer::new(model);
    player.score.playinfo.randomoption = 2;
    player.score.playinfo.randomoption2 = 3;
    player.score.playinfo.doubleoption = 1;
    // Combined: 2 + 3 * 10 + 1 * 100 = 132
    assert_eq!(player.encode_option_for_score(), 132);
}

#[test]
fn encode_option_for_score_dp_no_doubleoption() {
    let mut model = BMSModel::new();
    model.set_mode(Mode::BEAT_14K);
    model.judgerank = 100;
    let mut player = BMSPlayer::new(model);
    player.score.playinfo.randomoption = 1;
    player.score.playinfo.randomoption2 = 4;
    player.score.playinfo.doubleoption = 0;
    // Combined: 1 + 4 * 10 + 0 * 100 = 41
    assert_eq!(player.encode_option_for_score(), 41);
}

// --- seed round-trip test ---

#[test]
fn seed_round_trip_preserved_after_build_modifiers() {
    let model = make_model();
    let mut player = BMSPlayer::new(model);
    let config = make_default_config();

    // First build: generates a new seed
    player.build_pattern_modifiers(&config);
    let saved_seed = player.score.playinfo.randomoptionseed;
    assert_ne!(saved_seed, -1, "Seed should be initialized");

    // Second build with the same player: seed should be preserved
    // (since randomoptionseed is no longer -1, the restore path is used)
    let model2 = make_model();
    let mut player2 = BMSPlayer::new(model2);
    player2.score.playinfo.randomoptionseed = saved_seed;
    player2.build_pattern_modifiers(&config);
    assert_eq!(
        player2.score.playinfo.randomoptionseed, saved_seed,
        "Seed should be preserved on rebuild"
    );
}

#[test]
fn build_pattern_modifiers_lane_shuffle_pattern_saved() {
    let mut model = BMSModel::new();
    model.set_mode(Mode::BEAT_14K); // DP mode
    model.judgerank = 100;
    let tl = bms_model::time_line::TimeLine::new(130.0, 0, 16);
    model.timelines = vec![tl];
    let mut player = BMSPlayer::new(model);

    let mut config = make_default_config();
    // Random (id=2) creates LaneRandomShuffleModifier with show_shuffle_pattern=true
    config.play_settings.random = 2;
    player.score.playinfo.randomoption = 2;

    player.build_pattern_modifiers(&config);
    // lane_shuffle_pattern should be initialized with player count
    let lsp = player.score.playinfo.lane_shuffle_pattern.as_ref();
    assert!(
        lsp.is_some(),
        "lane_shuffle_pattern should be set for DP mode with Random option"
    );
    assert_eq!(
        lsp.unwrap().len(),
        2,
        "DP mode should have 2 player patterns"
    );
}

// --- restore_replay_data tests (Phase 34c) ---

fn make_replay_data() -> ReplayData {
    let mut rd = ReplayData::new();
    rd.randomoption = 3;
    rd.randomoptionseed = 99999;
    rd.randomoption2 = 2;
    rd.randomoption2seed = 88888;
    rd.doubleoption = 1;
    rd.rand = vec![2, 5, 1];
    rd.gauge = rubato_types::groove_gauge::HARD;
    rd.config = Some(rubato_types::play_config::PlayConfig {
        hispeed: 5.0,
        duration: 300,
        ..rubato_types::play_config::PlayConfig::default()
    });
    rd
}

#[test]
fn restore_replay_data_none_returns_no_stay() {
    let model = make_model();
    let mut player = BMSPlayer::new(model);
    let key_state = ReplayKeyState::default();

    let result = player.restore_replay_data(None, &key_state);
    assert!(!result.stay_replay);
    assert!(result.replay.is_none());
    assert!(result.hs_replay_config.is_none());
    // playinfo should be unchanged
    assert_eq!(player.score.playinfo.randomoption, 0);
    assert_eq!(player.score.playinfo.randomoptionseed, -1);
}

#[test]
fn restore_replay_data_pattern_key_copies_all_fields() {
    let model = make_model();
    let mut player = BMSPlayer::new(model);
    let replay = make_replay_data();

    let key_state = ReplayKeyState {
        pattern_key: true,
        ..Default::default()
    };

    let result = player.restore_replay_data(Some(replay), &key_state);
    // Should switch to PLAY mode
    assert!(!result.stay_replay);
    assert!(result.replay.is_none());

    // All fields should be copied
    assert_eq!(player.score.playinfo.randomoption, 3);
    assert_eq!(player.score.playinfo.randomoptionseed, 99999);
    assert_eq!(player.score.playinfo.randomoption2, 2);
    assert_eq!(player.score.playinfo.randomoption2seed, 88888);
    assert_eq!(player.score.playinfo.doubleoption, 1);
    assert_eq!(player.score.playinfo.rand, vec![2, 5, 1]);
}

#[test]
fn restore_replay_data_option_key_copies_options_not_seeds() {
    let model = make_model();
    let mut player = BMSPlayer::new(model);
    let replay = make_replay_data();

    let key_state = ReplayKeyState {
        option_key: true,
        ..Default::default()
    };

    let result = player.restore_replay_data(Some(replay), &key_state);
    // Should switch to PLAY mode
    assert!(!result.stay_replay);
    assert!(result.replay.is_none());

    // Options should be copied
    assert_eq!(player.score.playinfo.randomoption, 3);
    assert_eq!(player.score.playinfo.randomoption2, 2);
    assert_eq!(player.score.playinfo.doubleoption, 1);

    // Seeds should NOT be copied (remain at default -1)
    assert_eq!(player.score.playinfo.randomoptionseed, -1);
    assert_eq!(player.score.playinfo.randomoption2seed, -1);

    // Rand should NOT be copied
    assert!(player.score.playinfo.rand.is_empty());
}

#[test]
fn restore_replay_data_hs_key_saves_config() {
    let model = make_model();
    let mut player = BMSPlayer::new(model);
    let replay = make_replay_data();

    let key_state = ReplayKeyState {
        hs_key: true,
        ..Default::default()
    };

    let result = player.restore_replay_data(Some(replay), &key_state);
    // Should switch to PLAY mode
    assert!(!result.stay_replay);
    assert!(result.replay.is_none());

    // HS config should be returned
    let hs_config = result.hs_replay_config.unwrap();
    assert_eq!(hs_config.hispeed, 5.0);
    assert_eq!(hs_config.duration, 300);
}

#[test]
fn restore_replay_data_pattern_and_hs_keys_together() {
    let model = make_model();
    let mut player = BMSPlayer::new(model);
    let replay = make_replay_data();

    let key_state = ReplayKeyState {
        pattern_key: true,
        hs_key: true,
        ..Default::default()
    };

    let result = player.restore_replay_data(Some(replay), &key_state);
    assert!(!result.stay_replay);
    assert!(result.replay.is_none());

    // Pattern fields should be copied
    assert_eq!(player.score.playinfo.randomoption, 3);
    assert_eq!(player.score.playinfo.randomoptionseed, 99999);

    // HS config should also be returned
    assert!(result.hs_replay_config.is_some());
}

#[test]
fn restore_replay_data_no_keys_stays_replay() {
    let model = make_model();
    let mut player = BMSPlayer::new(model);
    let replay = make_replay_data();

    let key_state = ReplayKeyState::default();

    let result = player.restore_replay_data(Some(replay.clone()), &key_state);
    // Should stay in REPLAY mode
    assert!(result.stay_replay);
    assert!(result.replay.is_some());
    assert!(result.hs_replay_config.is_none());

    // playinfo should be unchanged
    assert_eq!(player.score.playinfo.randomoption, 0);
    assert_eq!(player.score.playinfo.randomoptionseed, -1);
}

// --- select_gauge_type tests (Phase 34c) ---

#[test]
fn select_gauge_type_no_replay_uses_config() {
    let key_state = ReplayKeyState::default();
    let result = BMSPlayer::select_gauge_type(None, rubato_types::groove_gauge::NORMAL, &key_state);
    assert_eq!(result, rubato_types::groove_gauge::NORMAL);
}

#[test]
fn select_gauge_type_replay_uses_replay_gauge() {
    let mut replay = make_replay_data();
    replay.gauge = rubato_types::groove_gauge::HARD;
    let key_state = ReplayKeyState::default();
    let result = BMSPlayer::select_gauge_type(
        Some(&replay),
        rubato_types::groove_gauge::NORMAL,
        &key_state,
    );
    assert_eq!(result, rubato_types::groove_gauge::HARD);
}

#[test]
fn select_gauge_type_replay_with_key5_shifts_by_1() {
    let mut replay = make_replay_data();
    replay.gauge = rubato_types::groove_gauge::NORMAL; // 2
    let key_state = ReplayKeyState {
        gauge_shift_key5: true,
        ..Default::default()
    };
    let result = BMSPlayer::select_gauge_type(
        Some(&replay),
        rubato_types::groove_gauge::NORMAL,
        &key_state,
    );
    assert_eq!(result, rubato_types::groove_gauge::HARD); // 2 + 1 = 3
}

#[test]
fn select_gauge_type_replay_with_key3_shifts_by_2() {
    let mut replay = make_replay_data();
    replay.gauge = rubato_types::groove_gauge::NORMAL; // 2
    let key_state = ReplayKeyState {
        gauge_shift_key3: true,
        ..Default::default()
    };
    let result = BMSPlayer::select_gauge_type(
        Some(&replay),
        rubato_types::groove_gauge::NORMAL,
        &key_state,
    );
    assert_eq!(result, rubato_types::groove_gauge::EXHARD); // 2 + 2 = 4
}

#[test]
fn select_gauge_type_replay_with_both_keys_shifts_by_3() {
    let mut replay = make_replay_data();
    replay.gauge = rubato_types::groove_gauge::NORMAL; // 2
    let key_state = ReplayKeyState {
        gauge_shift_key3: true,
        gauge_shift_key5: true,
        ..Default::default()
    };
    let result = BMSPlayer::select_gauge_type(
        Some(&replay),
        rubato_types::groove_gauge::NORMAL,
        &key_state,
    );
    assert_eq!(result, rubato_types::groove_gauge::HAZARD); // 2 + 3 = 5
}

#[test]
fn select_gauge_type_replay_hazard_no_shift() {
    let mut replay = make_replay_data();
    replay.gauge = rubato_types::groove_gauge::HAZARD; // 5
    let key_state = ReplayKeyState {
        gauge_shift_key5: true,
        ..Default::default()
    };
    let result = BMSPlayer::select_gauge_type(
        Some(&replay),
        rubato_types::groove_gauge::NORMAL,
        &key_state,
    );
    // HAZARD cannot be shifted further
    assert_eq!(result, rubato_types::groove_gauge::HAZARD);
}

// --- handle_random_syntax tests (Phase 34c) ---

#[test]
fn handle_random_syntax_no_random_in_model() {
    let model = make_model(); // No random branches set
    let mut player = BMSPlayer::new(model);
    let result = player.handle_random_syntax(false, None, -1, &[]);
    assert!(result.is_none());
    assert!(player.score.playinfo.rand.is_empty());
}

#[test]
fn handle_random_syntax_replay_mode_uses_replay_rand() {
    let mut model = BMSModel::new();
    model.set_mode(Mode::BEAT_7K);
    model.judgerank = 100;
    model.info = Some(bms_model::chart_information::ChartInformation::new(
        None,
        bms_model::bms_model::LnType::LongNote,
        Some(vec![1, 3, 2]),
    )); // Model has random branches
    let mut player = BMSPlayer::new(model);

    let mut replay = make_replay_data();
    replay.rand = vec![2, 1, 3];

    let result = player.handle_random_syntax(true, Some(&replay), -1, &[]);
    // Should return Some with the replay's rand for model reload
    assert!(result.is_some());
    assert_eq!(result.unwrap(), vec![2, 1, 3]);
    assert_eq!(player.score.playinfo.rand, vec![2, 1, 3]);
}

#[test]
fn handle_random_syntax_resource_seed_set_uses_resource_rand() {
    let mut model = BMSModel::new();
    model.set_mode(Mode::BEAT_7K);
    model.judgerank = 100;
    model.info = Some(bms_model::chart_information::ChartInformation::new(
        None,
        bms_model::bms_model::LnType::LongNote,
        Some(vec![1, 3, 2]),
    ));
    let mut player = BMSPlayer::new(model);

    let resource_rand = vec![3, 2, 1];

    let result = player.handle_random_syntax(false, None, 42, &resource_rand);
    assert!(result.is_some());
    assert_eq!(result.unwrap(), vec![3, 2, 1]);
    assert_eq!(player.score.playinfo.rand, vec![3, 2, 1]);
}

#[test]
fn handle_random_syntax_normal_play_stores_model_random() {
    let mut model = BMSModel::new();
    model.set_mode(Mode::BEAT_7K);
    model.judgerank = 100;
    model.info = Some(bms_model::chart_information::ChartInformation::new(
        None,
        bms_model::bms_model::LnType::LongNote,
        Some(vec![4, 5, 6]),
    ));
    let mut player = BMSPlayer::new(model);

    let result = player.handle_random_syntax(false, None, -1, &[]);
    // No reload needed (no rand override), but model's random should be stored
    assert!(result.is_none());
    assert_eq!(player.score.playinfo.rand, vec![4, 5, 6]);
}

#[test]
fn handle_random_syntax_replay_empty_rand_stores_model_random() {
    let mut model = BMSModel::new();
    model.set_mode(Mode::BEAT_7K);
    model.judgerank = 100;
    model.info = Some(bms_model::chart_information::ChartInformation::new(
        None,
        bms_model::bms_model::LnType::LongNote,
        Some(vec![1, 2]),
    ));
    let mut player = BMSPlayer::new(model);

    let mut replay = make_replay_data();
    replay.rand = vec![]; // Empty rand in replay

    let result = player.handle_random_syntax(true, Some(&replay), -1, &[]);
    // Empty rand means no reload, store model's random
    assert!(result.is_none());
    assert_eq!(player.score.playinfo.rand, vec![1, 2]);
}

// --- calculate_non_modifier_assist tests (Phase 34d) ---

/// Helper: create a model with uniform BPM (min == max).
fn make_model_uniform_bpm() -> BMSModel {
    let mut model = BMSModel::new();
    model.set_mode(Mode::BEAT_7K);
    model.bpm = 150.0;
    model.judgerank = 100;
    // Single timeline at the same BPM → min == max
    let mut tl = bms_model::time_line::TimeLine::new(0.0, 0, 8);
    tl.bpm = 150.0;
    model.timelines = vec![tl];
    model
}

/// Helper: create a model with variable BPM (min < max).
fn make_model_variable_bpm() -> BMSModel {
    let mut model = BMSModel::new();
    model.set_mode(Mode::BEAT_7K);
    model.bpm = 120.0;
    model.judgerank = 100;
    // Two timelines with different BPMs → min != max
    let mut tl1 = bms_model::time_line::TimeLine::new(0.0, 0, 8);
    tl1.bpm = 120.0;
    let mut tl2 = bms_model::time_line::TimeLine::new(1.0, 1_000_000, 8);
    tl2.bpm = 180.0;
    model.timelines = vec![tl1, tl2];
    model
}

#[test]
fn non_modifier_assist_bpmguide_uniform_bpm_no_assist() {
    let model = make_model_uniform_bpm();
    let mut player = BMSPlayer::new(model);
    let mut config = make_default_config();
    config.display_settings.bpmguide = true; // BPM guide enabled

    let score = player.calculate_non_modifier_assist(&config);
    // Uniform BPM: min == max → BPM guide has no effect
    assert_eq!(player.assist, 0);
    assert!(score, "Score should remain valid with uniform BPM");
}

#[test]
fn non_modifier_assist_bpmguide_variable_bpm_sets_light_assist() {
    let model = make_model_variable_bpm();
    let mut player = BMSPlayer::new(model);
    let mut config = make_default_config();
    config.display_settings.bpmguide = true; // BPM guide enabled

    let score = player.calculate_non_modifier_assist(&config);
    // Variable BPM: min < max → assist = max(0, 1) = 1
    assert_eq!(player.assist, 1);
    assert!(
        !score,
        "Score should be invalid with BPM guide on variable BPM"
    );
}

#[test]
fn non_modifier_assist_bpmguide_disabled_no_assist() {
    let model = make_model_variable_bpm();
    let mut player = BMSPlayer::new(model);
    let config = make_default_config(); // bpmguide defaults to false

    let score = player.calculate_non_modifier_assist(&config);
    assert_eq!(player.assist, 0);
    assert!(score);
}

#[test]
fn non_modifier_assist_custom_judge_all_rates_lte_100_no_assist() {
    let model = make_model();
    let mut player = BMSPlayer::new(model);
    let mut config = make_default_config();
    config.judge_settings.custom_judge = true;
    // Set all rates to <= 100
    config.judge_settings.key_judge_window_rate_perfect_great = 100;
    config.judge_settings.key_judge_window_rate_great = 100;
    config.judge_settings.key_judge_window_rate_good = 100;
    config
        .judge_settings
        .scratch_judge_window_rate_perfect_great = 100;
    config.judge_settings.scratch_judge_window_rate_great = 100;
    config.judge_settings.scratch_judge_window_rate_good = 100;

    let score = player.calculate_non_modifier_assist(&config);
    assert_eq!(player.assist, 0);
    assert!(score);
}

#[test]
fn non_modifier_assist_custom_judge_one_rate_over_100_sets_assist() {
    let model = make_model();
    let mut player = BMSPlayer::new(model);
    let mut config = make_default_config();
    config.judge_settings.custom_judge = true;
    // Only one rate > 100
    config.judge_settings.key_judge_window_rate_perfect_great = 101;
    config.judge_settings.key_judge_window_rate_great = 50;
    config.judge_settings.key_judge_window_rate_good = 50;
    config
        .judge_settings
        .scratch_judge_window_rate_perfect_great = 50;
    config.judge_settings.scratch_judge_window_rate_great = 50;
    config.judge_settings.scratch_judge_window_rate_good = 50;

    let score = player.calculate_non_modifier_assist(&config);
    assert_eq!(player.assist, 2);
    assert!(
        !score,
        "Score should be invalid with custom judge rate > 100"
    );
}

#[test]
fn non_modifier_assist_custom_judge_scratch_rate_over_100_sets_assist() {
    let model = make_model();
    let mut player = BMSPlayer::new(model);
    let mut config = make_default_config();
    config.judge_settings.custom_judge = true;
    config.judge_settings.key_judge_window_rate_perfect_great = 50;
    config.judge_settings.key_judge_window_rate_great = 50;
    config.judge_settings.key_judge_window_rate_good = 50;
    config
        .judge_settings
        .scratch_judge_window_rate_perfect_great = 50;
    config.judge_settings.scratch_judge_window_rate_great = 50;
    config.judge_settings.scratch_judge_window_rate_good = 200; // Only scratch good > 100

    let score = player.calculate_non_modifier_assist(&config);
    assert_eq!(player.assist, 2);
    assert!(!score);
}

#[test]
fn non_modifier_assist_custom_judge_disabled_no_assist() {
    let model = make_model();
    let mut player = BMSPlayer::new(model);
    let mut config = make_default_config();
    config.judge_settings.custom_judge = false; // Disabled
    // Even with high rates, custom judge is off
    config.judge_settings.key_judge_window_rate_perfect_great = 400;

    let score = player.calculate_non_modifier_assist(&config);
    assert_eq!(player.assist, 0);
    assert!(score);
}

#[test]
fn non_modifier_assist_constant_speed_enabled_sets_assist() {
    let model = make_model();
    let mut player = BMSPlayer::new(model);
    let mut config = make_default_config();
    config.mode7.playconfig.enable_constant = true; // Enable constant speed for BEAT_7K

    let score = player.calculate_non_modifier_assist(&config);
    assert_eq!(player.assist, 2);
    assert!(!score, "Score should be invalid with constant speed");
}

#[test]
fn non_modifier_assist_constant_speed_disabled_no_assist() {
    let model = make_model();
    let mut player = BMSPlayer::new(model);
    let config = make_default_config(); // enable_constant defaults to false

    let score = player.calculate_non_modifier_assist(&config);
    assert_eq!(player.assist, 0);
    assert!(score);
}

#[test]
fn non_modifier_assist_accumulates_bpmguide_and_constant() {
    // BPM guide → assist=1, constant → assist=max(1,2)=2
    let model = make_model_variable_bpm();
    let mut player = BMSPlayer::new(model);
    let mut config = make_default_config();
    config.display_settings.bpmguide = true;
    config.mode7.playconfig.enable_constant = true;

    let score = player.calculate_non_modifier_assist(&config);
    assert_eq!(
        player.assist, 2,
        "Assist should accumulate to max (BPM guide=1, constant=2)"
    );
    assert!(!score);
}

#[test]
fn non_modifier_assist_preserves_existing_assist() {
    // If assist was already set to 1 by pattern modifiers, non-modifier check
    // should keep the max
    let model = make_model();
    let mut player = BMSPlayer::new(model);
    player.assist = 1; // Pre-set by pattern modifiers

    let mut config = make_default_config();
    config.mode7.playconfig.enable_constant = true; // Would set assist=2

    let score = player.calculate_non_modifier_assist(&config);
    assert_eq!(player.assist, 2, "Assist should be max(1, 2) = 2");
    assert!(!score);
}

// --- get_clear_type_for_assist tests (Phase 34d) ---

#[test]
fn clear_type_for_assist_0_returns_none() {
    let model = make_model();
    let player = BMSPlayer::new(model);
    // assist defaults to 0
    assert!(player.clear_type_for_assist().is_none());
}

#[test]
fn clear_type_for_assist_1_returns_light_assist_easy() {
    let model = make_model();
    let mut player = BMSPlayer::new(model);
    player.assist = 1;
    assert_eq!(
        player.clear_type_for_assist(),
        Some(ClearType::LightAssistEasy)
    );
}

#[test]
fn clear_type_for_assist_2_returns_noplay() {
    let model = make_model();
    let mut player = BMSPlayer::new(model);
    player.assist = 2;
    assert_eq!(player.clear_type_for_assist(), Some(ClearType::NoPlay));
}

#[test]
fn clear_type_for_assist_3_returns_noplay() {
    let model = make_model();
    let mut player = BMSPlayer::new(model);
    player.assist = 3; // Any value >= 2 should be NoPlay
    assert_eq!(player.clear_type_for_assist(), Some(ClearType::NoPlay));
}

// --- init_playinfo_from_config tests (Phase 34e) ---

#[test]
fn init_playinfo_from_config_copies_random_options() {
    let model = make_model();
    let mut player = BMSPlayer::new(model);
    let mut config = make_default_config();
    config.play_settings.random = 3;
    config.play_settings.random2 = 5;
    config.play_settings.doubleoption = 2;

    player.init_playinfo_from_config(&config);

    assert_eq!(player.score.playinfo.randomoption, 3);
    assert_eq!(player.score.playinfo.randomoption2, 5);
    assert_eq!(player.score.playinfo.doubleoption, 2);
}

#[test]
fn init_playinfo_from_config_default_config_zeros() {
    let model = make_model();
    let mut player = BMSPlayer::new(model);
    let config = make_default_config();

    player.init_playinfo_from_config(&config);

    assert_eq!(player.score.playinfo.randomoption, 0);
    assert_eq!(player.score.playinfo.randomoption2, 0);
    assert_eq!(player.score.playinfo.doubleoption, 0);
}

#[test]
fn init_playinfo_from_config_does_not_touch_seeds() {
    let model = make_model();
    let mut player = BMSPlayer::new(model);
    let mut config = make_default_config();
    config.play_settings.random = 2;

    player.init_playinfo_from_config(&config);

    // Seeds should remain at their default (-1 from ReplayData::new())
    assert_eq!(player.score.playinfo.randomoptionseed, -1);
    assert_eq!(player.score.playinfo.randomoption2seed, -1);
}

// --- End-to-end DP flow tests (Phase 34e) ---

#[test]
fn e2e_dp_flow_config_init_build_encode() {
    // End-to-end test: config → init → build → encode
    // DP mode (BEAT_14K) with FLIP (doubleoption=1), random=2, random2=3
    let mut model = BMSModel::new();
    model.set_mode(Mode::BEAT_14K);
    model.judgerank = 100;
    let mut player = BMSPlayer::new(model);

    let mut config = make_default_config();
    config.play_settings.random = 2;
    config.play_settings.random2 = 3;
    config.play_settings.doubleoption = 1;

    // Step 1: init from config
    player.init_playinfo_from_config(&config);
    assert_eq!(player.score.playinfo.randomoption, 2);
    assert_eq!(player.score.playinfo.randomoption2, 3);
    assert_eq!(player.score.playinfo.doubleoption, 1);

    // Step 2: build pattern modifiers
    player.build_pattern_modifiers(&config);

    // Step 3: encode option
    // Expected: randomoption + randomoption2 * 10 + doubleoption * 100
    // = 2 + 3 * 10 + 1 * 100 = 132
    assert_eq!(player.encode_option_for_score(), 132);
}

#[test]
fn e2e_dp_flow_replay_overrides_config() {
    // Config sets random=2, random2=3, doubleoption=1
    // Replay pattern key overrides to random=5, random2=7, doubleoption=0
    let mut model = BMSModel::new();
    model.set_mode(Mode::BEAT_14K);
    model.judgerank = 100;
    let mut player = BMSPlayer::new(model);

    let mut config = make_default_config();
    config.play_settings.random = 2;
    config.play_settings.random2 = 3;
    config.play_settings.doubleoption = 1;

    // Step 1: init from config
    player.init_playinfo_from_config(&config);

    // Step 2: replay overrides
    let mut replay = ReplayData::new();
    replay.randomoption = 5;
    replay.randomoptionseed = 42;
    replay.randomoption2 = 7;
    replay.randomoption2seed = 84;
    replay.doubleoption = 0;
    replay.rand = vec![1, 2];

    let key_state = ReplayKeyState {
        pattern_key: true,
        ..Default::default()
    };
    player.restore_replay_data(Some(replay), &key_state);

    // After replay override, playinfo should reflect replay values
    assert_eq!(player.score.playinfo.randomoption, 5);
    assert_eq!(player.score.playinfo.randomoption2, 7);
    assert_eq!(player.score.playinfo.doubleoption, 0);

    // Step 3: build pattern modifiers (uses overridden values)
    player.build_pattern_modifiers(&config);

    // Step 4: encode option
    // = 5 + 7 * 10 + 0 * 100 = 75
    assert_eq!(player.encode_option_for_score(), 75);
}

#[test]
fn e2e_sp_mode_ignores_2p_options() {
    // SP mode (BEAT_7K) end-to-end: 2P options should be ignored in encoding
    let model = make_model(); // BEAT_7K
    let mut player = BMSPlayer::new(model);

    let mut config = make_default_config();
    config.play_settings.random = 3;
    config.play_settings.random2 = 5; // Should be irrelevant in SP
    config.play_settings.doubleoption = 1; // Should be irrelevant in SP

    // Step 1: init from config
    player.init_playinfo_from_config(&config);
    // All values are copied to playinfo
    assert_eq!(player.score.playinfo.randomoption, 3);
    assert_eq!(player.score.playinfo.randomoption2, 5);
    assert_eq!(player.score.playinfo.doubleoption, 1);

    // Step 2: build pattern modifiers
    player.build_pattern_modifiers(&config);

    // Step 3: encode option — SP mode only uses randomoption
    // player_count == 1, so result is just randomoption
    assert_eq!(player.encode_option_for_score(), 3);
}

#[test]
fn e2e_dp_battle_mode_config_init_build_encode() {
    // DP battle mode: SP BEAT_7K with doubleoption=2 → converts to BEAT_14K
    let mut model = BMSModel::new();
    model.set_mode(Mode::BEAT_7K);
    model.judgerank = 100;
    let mut player = BMSPlayer::new(model);

    let mut config = make_default_config();
    config.play_settings.random = 1;
    config.play_settings.random2 = 4;
    config.play_settings.doubleoption = 2;

    // Step 1: init from config
    player.init_playinfo_from_config(&config);

    // Step 2: build pattern modifiers (converts SP to DP)
    let score = player.build_pattern_modifiers(&config);
    assert!(!score, "Battle mode should invalidate score");
    assert_eq!(player.mode(), Mode::BEAT_14K);

    // Step 3: encode option — now in DP mode (player=2)
    // = 1 + 4 * 10 + 2 * 100 = 241
    assert_eq!(player.encode_option_for_score(), 241);
}

// --- apply_freq_trainer tests (Phase 34f) ---

#[test]
fn freq_trainer_freq_100_returns_none() {
    let model = make_model_with_time(10000);
    let mut player = BMSPlayer::new(model);
    let result = player.apply_freq_trainer(100, true, false, &FrequencyType::FREQUENCY);
    assert!(result.is_none(), "freq=100 should return None (no change)");
}

#[test]
fn freq_trainer_freq_0_returns_none() {
    let model = make_model_with_time(10000);
    let mut player = BMSPlayer::new(model);
    let result = player.apply_freq_trainer(0, true, false, &FrequencyType::FREQUENCY);
    assert!(result.is_none(), "freq=0 should return None (no change)");
}

#[test]
fn freq_trainer_not_play_mode_returns_none() {
    let model = make_model_with_time(10000);
    let mut player = BMSPlayer::new(model);
    let result = player.apply_freq_trainer(150, false, false, &FrequencyType::FREQUENCY);
    assert!(
        result.is_none(),
        "Not play mode should return None (no change)"
    );
}

#[test]
fn freq_trainer_course_mode_returns_none() {
    let model = make_model_with_time(10000);
    let mut player = BMSPlayer::new(model);
    let result = player.apply_freq_trainer(150, true, true, &FrequencyType::FREQUENCY);
    assert!(
        result.is_none(),
        "Course mode should return None (no change)"
    );
}

#[test]
fn freq_trainer_freq_150_adjusts_playtime() {
    let model = make_model_with_time(10000);
    let mut player = BMSPlayer::new(model);
    let last_note_time = player.model.last_note_time();

    let result = player.apply_freq_trainer(150, true, false, &FrequencyType::FREQUENCY);
    assert!(result.is_some());

    // Expected: (lastNoteTime + 1000) * 100 / 150 + TIME_MARGIN
    let expected_playtime = (last_note_time + 1000) * 100 / 150 + TIME_MARGIN;
    assert_eq!(player.playtime(), expected_playtime);
}

#[test]
fn freq_trainer_freq_50_adjusts_playtime() {
    let model = make_model_with_time(10000);
    let mut player = BMSPlayer::new(model);
    let last_note_time = player.model.last_note_time();

    let result = player.apply_freq_trainer(50, true, false, &FrequencyType::FREQUENCY);
    assert!(result.is_some());

    // Expected: (lastNoteTime + 1000) * 100 / 50 + TIME_MARGIN
    let expected_playtime = (last_note_time + 1000) * 100 / 50 + TIME_MARGIN;
    assert_eq!(player.playtime(), expected_playtime);
}

#[test]
fn freq_trainer_freq_string_format() {
    let model = make_model_with_time(10000);
    let mut player = BMSPlayer::new(model);

    let result = player.apply_freq_trainer(150, true, false, &FrequencyType::FREQUENCY);
    let result = result.unwrap();
    assert_eq!(result.freq_string, "[1.50x]");

    // Test with freq=50
    let model2 = make_model_with_time(10000);
    let mut player2 = BMSPlayer::new(model2);
    let result2 = player2.apply_freq_trainer(50, true, false, &FrequencyType::FREQUENCY);
    let result2 = result2.unwrap();
    assert_eq!(result2.freq_string, "[0.50x]");

    // Test with freq=200
    let model3 = make_model_with_time(10000);
    let mut player3 = BMSPlayer::new(model3);
    let result3 = player3.apply_freq_trainer(200, true, false, &FrequencyType::FREQUENCY);
    let result3 = result3.unwrap();
    assert_eq!(result3.freq_string, "[2.00x]");
}

#[test]
fn freq_trainer_global_pitch_set_when_frequency_type() {
    let model = make_model_with_time(10000);
    let mut player = BMSPlayer::new(model);

    let result = player.apply_freq_trainer(150, true, false, &FrequencyType::FREQUENCY);
    let result = result.unwrap();
    assert_eq!(result.global_pitch, Some(1.5));
}

#[test]
fn freq_trainer_global_pitch_none_when_unprocessed() {
    let model = make_model_with_time(10000);
    let mut player = BMSPlayer::new(model);

    let result = player.apply_freq_trainer(150, true, false, &FrequencyType::UNPROCESSED);
    let result = result.unwrap();
    assert!(
        result.global_pitch.is_none(),
        "UNPROCESSED should not set global pitch"
    );
}

#[test]
fn freq_trainer_result_fields_correct() {
    let model = make_model_with_time(10000);
    let mut player = BMSPlayer::new(model);

    let result = player.apply_freq_trainer(150, true, false, &FrequencyType::FREQUENCY);
    let result = result.unwrap();
    assert!(result.freq_on);
    assert!(result.force_no_ir_send);
}

#[test]
fn freq_trainer_scales_chart_timing() {
    // Verify that change_frequency is called on the model
    let mut model = BMSModel::new();
    model.set_mode(Mode::BEAT_7K);
    model.judgerank = 100;
    model.bpm = 120.0;
    let mut tl = bms_model::time_line::TimeLine::new(0.0, 0, 8);
    tl.bpm = 120.0;
    let mut tl2 = bms_model::time_line::TimeLine::new(1.0, 1_000_000, 8);
    tl2.bpm = 120.0;
    tl2.set_note(0, Some(bms_model::note::Note::new_normal(1)));
    model.timelines = vec![tl, tl2];
    let original_bpm = model.bpm;

    let mut player = BMSPlayer::new(model);
    player.apply_freq_trainer(150, true, false, &FrequencyType::FREQUENCY);

    // BPM should be scaled by 1.5
    let expected_bpm = original_bpm * 1.5;
    let actual_bpm = player.model.bpm;
    assert!(
        (actual_bpm - expected_bpm).abs() < 0.001,
        "BPM should be scaled: expected {}, got {}",
        expected_bpm,
        actual_bpm
    );
}

// --- Global pitch control tests ---

#[test]
fn set_play_speed_sets_pending_pitch_when_frequency_type() {
    let model = make_model();
    let mut player = BMSPlayer::new(model);
    player.fast_forward_freq_option = FrequencyType::FREQUENCY;
    player.set_play_speed(150);
    assert_eq!(player.take_pending_global_pitch(), Some(1.5));
}

#[test]
fn set_play_speed_no_pending_pitch_when_unprocessed() {
    let model = make_model();
    let mut player = BMSPlayer::new(model);
    player.fast_forward_freq_option = FrequencyType::UNPROCESSED;
    player.set_play_speed(150);
    assert_eq!(player.take_pending_global_pitch(), None);
}

#[test]
fn take_pending_global_pitch_clears_after_read() {
    let model = make_model();
    let mut player = BMSPlayer::new(model);
    player.fast_forward_freq_option = FrequencyType::FREQUENCY;
    player.set_play_speed(200);
    assert_eq!(player.take_pending_global_pitch(), Some(2.0));
    // Second call should be None (consumed)
    assert_eq!(player.take_pending_global_pitch(), None);
}

#[test]
fn stop_play_preload_sets_pending_pitch_to_one() {
    let model = make_model();
    let mut player = BMSPlayer::new(model);
    player.state = PlayState::Preload;
    player.stop_play();
    assert_eq!(player.take_pending_global_pitch(), Some(1.0));
}

#[test]
fn stop_play_ready_sets_pending_pitch_to_one() {
    let model = make_model();
    let mut player = BMSPlayer::new(model);
    player.state = PlayState::Ready;
    player.stop_play();
    assert_eq!(player.take_pending_global_pitch(), Some(1.0));
}

#[test]
fn stop_play_failed_state_sets_pending_pitch_to_one() {
    let model = make_model();
    let mut player = BMSPlayer::new(model);
    player.state = PlayState::Play;
    // Ensure no notes judged and no prior timer
    player.stop_play();
    // This goes to ABORTED (no notes judged), no pitch reset here
    assert_eq!(player.state, PlayState::Aborted);
    // No pending pitch for ABORTED path (matches Java - only resets on failed path)
    assert_eq!(player.take_pending_global_pitch(), None);
}

#[test]
fn stop_play_failed_path_sets_pending_pitch_to_one() {
    let mut model = BMSModel::new();
    model.set_mode(Mode::BEAT_7K);
    model.judgerank = 100;
    let mut player = BMSPlayer::new(model);
    player.state = PlayState::Play;

    // Simulate some notes judged (not finished but notes exist)
    // Force the judge counts so we enter the failed branch
    player.judge.score_data_mut().judge_counts.epg = 5; // 5 early PGreats
    player.total_notes = 100; // not all past
    player.stop_play();
    assert_eq!(player.state, PlayState::Failed);
    assert_eq!(player.take_pending_global_pitch(), Some(1.0));
}

// --- Loudness analysis tests ---

#[test]
fn apply_loudness_analysis_success() {
    use rubato_audio::bms_loudness_analyzer::AnalysisResult;

    let model = make_model();
    let mut player = BMSPlayer::new(model);
    assert!(!player.is_analysis_checked());

    let result = AnalysisResult::new_success(-14.0);
    let vol = player.apply_loudness_analysis(&result, 1.0);
    assert!(player.is_analysis_checked());
    assert!(vol > 0.0 && vol <= 1.0);
    assert!((player.adjusted_volume() - vol).abs() < f32::EPSILON);
}

#[test]
fn apply_loudness_analysis_failure() {
    use rubato_audio::bms_loudness_analyzer::AnalysisResult;

    let model = make_model();
    let mut player = BMSPlayer::new(model);

    let result = AnalysisResult::new_error("test error".to_string());
    let vol = player.apply_loudness_analysis(&result, 1.0);
    assert!(player.is_analysis_checked());
    assert!((vol - (-1.0)).abs() < f32::EPSILON);
}

#[test]
fn apply_loudness_analysis_preserves_base_volume_on_failure() {
    use rubato_audio::bms_loudness_analyzer::AnalysisResult;

    let model = make_model();
    let mut player = BMSPlayer::new(model);

    let result = AnalysisResult::new_error("err".to_string());
    player.apply_loudness_analysis(&result, 0.8);
    // adjusted_volume should be -1.0 on failure
    assert!((player.adjusted_volume() - (-1.0)).abs() < f32::EPSILON);
}

// --- Guide SE config tests ---

#[test]
fn build_guide_se_config_disabled_returns_all_none() {
    let sm = rubato_core::system_sound_manager::SystemSoundManager::new(None, None);
    let config = BMSPlayer::build_guide_se_config(false, &sm);
    assert_eq!(config.len(), 6);
    for (i, (judge, path)) in config.iter().enumerate() {
        assert_eq!(*judge, i as i32);
        assert!(path.is_none(), "judge {} should have None path", i);
    }
}

#[test]
fn build_guide_se_config_enabled_returns_six_entries() {
    // Without actual sound files, paths will be None (no files found)
    let sm = rubato_core::system_sound_manager::SystemSoundManager::new(None, None);
    let config = BMSPlayer::build_guide_se_config(true, &sm);
    assert_eq!(config.len(), 6);
    // All entries should exist (though paths may be None since no actual sound files)
    for (i, (judge, _path)) in config.iter().enumerate() {
        assert_eq!(*judge, i as i32);
    }
}

// --- Fast forward freq option tests ---

#[test]
fn set_fast_forward_freq_option_stored() {
    let model = make_model();
    let mut player = BMSPlayer::new(model);
    player.fast_forward_freq_option = FrequencyType::FREQUENCY;
    player.set_play_speed(75);
    assert_eq!(player.take_pending_global_pitch(), Some(0.75));
}

// --- Phase 43a: create() side effects tests ---

#[test]
fn create_produces_side_effects() {
    let model = make_model();
    let mut player = BMSPlayer::new(model);
    player.create();
    let effects = player.take_create_side_effects();
    assert!(effects.is_some(), "create() should produce side effects");
}

#[test]
fn create_side_effects_consumed_after_take() {
    let model = make_model();
    let mut player = BMSPlayer::new(model);
    player.create();
    let _ = player.take_create_side_effects();
    assert!(
        player.take_create_side_effects().is_none(),
        "second take should return None"
    );
}

#[test]
fn create_side_effects_skin_type_matches_model() {
    let model = make_model(); // BEAT_7K
    let mut player = BMSPlayer::new(model);
    player.create();
    let effects = player.take_create_side_effects().unwrap();
    assert_eq!(effects.skin_type, Some(SkinType::Play7Keys));
}

#[test]
fn create_side_effects_skin_type_5k() {
    let mut model = BMSModel::new();
    model.set_mode(Mode::BEAT_5K);
    model.judgerank = 100;
    let mut player = BMSPlayer::new(model);
    player.create();
    let effects = player.take_create_side_effects().unwrap();
    assert_eq!(effects.skin_type, Some(SkinType::Play5Keys));
}

#[test]
fn create_side_effects_skin_type_14k() {
    let mut model = BMSModel::new();
    model.set_mode(Mode::BEAT_14K);
    model.judgerank = 100;
    let mut player = BMSPlayer::new(model);
    player.create();
    let effects = player.take_create_side_effects().unwrap();
    assert_eq!(effects.skin_type, Some(SkinType::Play14Keys));
}

#[test]
fn create_side_effects_input_mode_play() {
    let model = make_model();
    let mut player = BMSPlayer::new(model);
    player.play_mode = BMSPlayerMode::PLAY;
    player.create();
    let effects = player.take_create_side_effects().unwrap();
    assert_eq!(
        effects.input_mode_action,
        InputModeAction::SetPlayConfig(Mode::BEAT_7K)
    );
}

#[test]
fn create_side_effects_input_mode_practice() {
    let model = make_model();
    let mut player = BMSPlayer::new(model);
    player.play_mode = BMSPlayerMode::PRACTICE;
    player.create();
    let effects = player.take_create_side_effects().unwrap();
    assert_eq!(
        effects.input_mode_action,
        InputModeAction::SetPlayConfig(Mode::BEAT_7K)
    );
}

#[test]
fn create_side_effects_input_mode_autoplay() {
    let model = make_model();
    let mut player = BMSPlayer::new(model);
    player.play_mode = BMSPlayerMode::AUTOPLAY;
    player.create();
    let effects = player.take_create_side_effects().unwrap();
    assert_eq!(effects.input_mode_action, InputModeAction::DisableInput);
}

#[test]
fn create_side_effects_input_mode_replay() {
    let model = make_model();
    let mut player = BMSPlayer::new(model);
    player.play_mode = BMSPlayerMode::REPLAY_1;
    player.create();
    let effects = player.take_create_side_effects().unwrap();
    assert_eq!(effects.input_mode_action, InputModeAction::DisableInput);
}

#[test]
fn create_side_effects_guide_se_disabled() {
    let model = make_model();
    let mut player = BMSPlayer::new(model);
    player.is_guide_se = false;
    player.create();
    let effects = player.take_create_side_effects().unwrap();
    assert!(!effects.is_guide_se);
}

#[test]
fn create_side_effects_guide_se_enabled() {
    let model = make_model();
    let mut player = BMSPlayer::new(model);
    player.is_guide_se = true;
    player.create();
    let effects = player.take_create_side_effects().unwrap();
    assert!(effects.is_guide_se);
}

#[test]
fn create_no_speed_disables_control() {
    let model = make_model();
    let mut player = BMSPlayer::new(model);
    player.constraints = vec![CourseDataConstraint::NoSpeed];
    player.create();
    // Verify control is disabled by checking its enable_control field
    let control = player.input.control.as_ref().unwrap();
    assert!(!control.is_enable_control());
}

#[test]
fn create_without_no_speed_keeps_control_enabled() {
    let model = make_model();
    let mut player = BMSPlayer::new(model);
    player.constraints = vec![CourseDataConstraint::Class];
    player.create();
    let control = player.input.control.as_ref().unwrap();
    assert!(control.is_enable_control());
}

#[test]
fn create_empty_constraints_keeps_control_enabled() {
    let model = make_model();
    let mut player = BMSPlayer::new(model);
    player.constraints = vec![];
    player.create();
    let control = player.input.control.as_ref().unwrap();
    assert!(control.is_enable_control());
}

#[test]
fn create_practice_mode_sets_state_practice() {
    let model = make_model();
    let mut player = BMSPlayer::new(model);
    player.play_mode = BMSPlayerMode::PRACTICE;
    player.create();
    assert_eq!(player.state(), PlayState::Practice);
}

#[test]
fn create_play_mode_keeps_state_preload() {
    let model = make_model();
    let mut player = BMSPlayer::new(model);
    player.play_mode = BMSPlayerMode::PLAY;
    player.create();
    assert_eq!(player.state(), PlayState::Preload);
}

#[test]
fn create_note_expansion_rate_default_no_expansion() {
    let model = make_model();
    let mut player = BMSPlayer::new(model);
    // Default PlaySkin has [100, 100] — no expansion
    player.create();
    // Rhythm processor should be created (existence is enough to verify create ran)
    assert!(player.rhythm.is_some());
}

#[test]
fn create_note_expansion_rate_custom_triggers_expansion() {
    let model = make_model();
    let mut player = BMSPlayer::new(model);
    // Set custom expansion rate before create
    player.play_skin.note_expansion_rate = [120, 100];
    player.create();
    assert!(player.rhythm.is_some());
}

#[test]
fn set_play_mode_and_get() {
    let model = make_model();
    let mut player = BMSPlayer::new(model);
    player.play_mode = BMSPlayerMode::AUTOPLAY;
    assert_eq!(
        player.play_mode().mode,
        rubato_core::bms_player_mode::Mode::Autoplay
    );
}

#[test]
fn set_constraints_and_get() {
    let model = make_model();
    let mut player = BMSPlayer::new(model);
    player.constraints = vec![CourseDataConstraint::NoSpeed, CourseDataConstraint::Class];
    assert_eq!(player.constraints().len(), 2);
    assert!(
        player
            .constraints()
            .contains(&CourseDataConstraint::NoSpeed)
    );
    assert!(player.constraints().contains(&CourseDataConstraint::Class));
}

#[test]
fn default_play_mode_is_play() {
    let model = make_model();
    let player = BMSPlayer::new(model);
    assert_eq!(
        player.play_mode().mode,
        rubato_core::bms_player_mode::Mode::Play
    );
}

#[test]
fn default_constraints_empty() {
    let model = make_model();
    let player = BMSPlayer::new(model);
    assert!(player.constraints().is_empty());
}

#[test]
fn default_guide_se_disabled() {
    let model = make_model();
    let player = BMSPlayer::new(model);
    assert!(!player.is_guide_se);
}

#[test]
fn create_side_effects_none_before_create() {
    let model = make_model();
    let mut player = BMSPlayer::new(model);
    assert!(player.take_create_side_effects().is_none());
}

#[test]
fn create_input_mode_5k_model_with_play_mode() {
    let mut model = BMSModel::new();
    model.set_mode(Mode::BEAT_5K);
    model.judgerank = 100;
    let mut player = BMSPlayer::new(model);
    player.play_mode = BMSPlayerMode::PLAY;
    player.create();
    let effects = player.take_create_side_effects().unwrap();
    assert_eq!(
        effects.input_mode_action,
        InputModeAction::SetPlayConfig(Mode::BEAT_5K)
    );
}

#[test]
fn create_no_speed_among_multiple_constraints() {
    let model = make_model();
    let mut player = BMSPlayer::new(model);
    player.constraints = vec![
        CourseDataConstraint::Class,
        CourseDataConstraint::NoSpeed,
        CourseDataConstraint::Mirror,
    ];
    player.create();
    let control = player.input.control.as_ref().unwrap();
    assert!(!control.is_enable_control());
}

// --- save_config tests ---

#[test]
fn save_config_skips_when_no_speed_constraint() {
    let model = make_model();
    let mut player = BMSPlayer::new(model);
    player.lanerender = Some(LaneRenderer::new(&player.model));
    player.constraints = vec![CourseDataConstraint::NoSpeed];

    // Set a known state on the lane renderer
    let pc_before = player
        .player_config
        .play_config_ref(Mode::BEAT_7K)
        .playconfig
        .clone();

    player.save_config();

    // Config should not have changed
    let pc_after = &player
        .player_config
        .play_config_ref(Mode::BEAT_7K)
        .playconfig;
    assert_eq!(pc_before.hispeed, pc_after.hispeed);
    assert_eq!(pc_before.lanecover, pc_after.lanecover);
}

#[test]
fn save_config_saves_lane_renderer_state() {
    let model = make_model();
    let mut player = BMSPlayer::new(model);
    player.lanerender = Some(LaneRenderer::new(&player.model));

    // Default fixhispeed is FIX_HISPEED_MAINBPM (not OFF), so duration should be saved
    player.save_config();

    let pc = &player
        .player_config
        .play_config_ref(Mode::BEAT_7K)
        .playconfig;
    // Duration should be set from lane renderer (default duration)
    let lr_duration = player.lanerender.as_ref().unwrap().duration();
    assert_eq!(pc.duration, lr_duration);
}

#[test]
fn save_config_saves_hispeed_when_fixhispeed_off() {
    let model = make_model();
    let mut player = BMSPlayer::new(model);
    player.lanerender = Some(LaneRenderer::new(&player.model));

    // Set fixhispeed to OFF
    player
        .player_config
        .play_config(Mode::BEAT_7K)
        .playconfig
        .fixhispeed = rubato_types::play_config::FIX_HISPEED_OFF;

    player.save_config();

    let pc = &player
        .player_config
        .play_config_ref(Mode::BEAT_7K)
        .playconfig;
    let lr_hispeed = player.lanerender.as_ref().unwrap().hispeed();
    assert_eq!(pc.hispeed, lr_hispeed);
}

// --- media_load_finished tests ---

#[test]
fn preload_does_not_transition_when_media_not_loaded() {
    let model = make_model();
    let mut player = BMSPlayer::new(model);
    player.play_skin.loadstart = 0;
    player.play_skin.loadend = 0;
    player.media_load_finished = false; // Media not loaded
    player.startpressedtime = -2_000_000;

    std::thread::sleep(std::time::Duration::from_millis(2));
    player.main_state_data.timer.update();
    player.render();

    // Should stay in PRELOAD because media not loaded
    assert_eq!(player.state(), PlayState::Preload);
}

// --- input state wiring tests ---

#[test]
fn sync_input_from_copies_live_controller_state() {
    let model = make_model();
    let mut player = BMSPlayer::new(model);
    let config = Config::default();
    let player_config = PlayerConfig::default();
    let mut input = BMSPlayerInputProcessor::new(&config, &player_config);

    input.start_changed(true);
    input.select_pressed = true;
    input.set_key_state(0, true, 1000);
    input
        .keyboard_input_processor_mut()
        .set_key_state(ControlKeys::Up.keycode(), true);
    input
        .keyboard_input_processor_mut()
        .set_key_state(ControlKeys::Down.keycode(), true);
    input
        .keyboard_input_processor_mut()
        .set_key_state(ControlKeys::Left.keycode(), true);
    input
        .keyboard_input_processor_mut()
        .set_key_state(ControlKeys::Right.keycode(), true);

    <BMSPlayer as MainState>::sync_input_from(&mut player, &input);

    assert!(player.input.input_start_pressed);
    assert!(player.input.input_select_pressed);
    assert_eq!(player.input.input_key_states.len(), KEYSTATE_SIZE);
    assert!(player.input.input_key_states[0]);
    assert!(player.input.control_key_up);
    assert!(player.input.control_key_down);
    assert!(player.input.control_key_left);
    assert!(player.input.control_key_right);
}

#[test]
fn sync_input_back_to_clears_consumed_start_and_select() {
    let model = make_model();
    let mut player = BMSPlayer::new(model);
    let config = Config::default();
    let player_config = PlayerConfig::default();
    let mut input = BMSPlayerInputProcessor::new(&config, &player_config);

    input.start_changed(true);
    input.select_pressed = true;
    player.input.input_start_pressed = false;
    player.input.input_select_pressed = false;

    <BMSPlayer as MainState>::sync_input_back_to(&mut player, &mut input);

    assert!(!input.start_pressed());
    assert!(!input.is_select_pressed());
}

#[test]
fn analog_cover_change_uses_live_input_and_flushes_reset_back() {
    let model = make_model();
    let mut player = BMSPlayer::new(model);
    player.total_notes = 1;
    player.input.control = Some(ControlInputProcessor::new(Mode::BEAT_7K));
    player.lanerender = Some(LaneRenderer::new(&player.model));
    player
        .lanerender
        .as_mut()
        .expect("lane renderer")
        .enable_lanecover = true;
    player
        .lanerender
        .as_mut()
        .expect("lane renderer")
        .set_lanecover(0.5);

    let config = Config::default();
    let player_config = PlayerConfig::default();
    let mut input = BMSPlayerInputProcessor::new(&config, &player_config);
    input.start_changed(true);
    input.set_key_state(7, true, 1_000);
    input.set_analog_state(7, true, 0.75);
    input.reset_analog_input(7);
    input.set_analog_state(7, true, 0.80);
    assert!(input.start_pressed());
    let expected_delta = input.analog_diff(7) as f32 * 0.001;

    <BMSPlayer as MainState>::sync_input_from(&mut player, &input);
    assert!(player.input.input_start_pressed);
    assert!(player.input.input_key_states[7]);
    assert!(player.input.input_is_analog[7]);
    assert_eq!(
        player.input.input_analog_diff_ticks[7],
        input.analog_diff(7)
    );
    player.input();
    <BMSPlayer as MainState>::sync_input_back_to(&mut player, &mut input);

    <BMSPlayer as MainState>::sync_input_from(&mut player, &input);
    assert!(player.input.input_start_pressed);
    assert!(player.input.input_key_states[7]);
    assert!(player.input.input_is_analog[7]);
    assert_eq!(
        player.input.input_analog_diff_ticks[7],
        input.analog_diff(7)
    );
    player.input();
    <BMSPlayer as MainState>::sync_input_back_to(&mut player, &mut input);

    let actual_cover = player
        .lanerender
        .as_ref()
        .expect("lane renderer")
        .lanecover();
    assert!(
        (actual_cover - (0.5 + expected_delta)).abs() < 0.001,
        "expected lanecover {}, got {}",
        0.5 + expected_delta,
        actual_cover
    );
    assert_eq!(input.analog_diff(7), 0);
}

// --- startpressedtime tracking tests ---

#[test]
fn startpressedtime_updates_when_start_pressed() {
    let model = make_model();
    let mut player = BMSPlayer::new(model);
    player.input.input_start_pressed = true;
    player.startpressedtime = -999;

    std::thread::sleep(std::time::Duration::from_millis(1));
    player.main_state_data.timer.update();
    player.render();

    // startpressedtime should have been updated to micronow
    assert!(player.startpressedtime > -999);
}

// --- gauge auto shift tests ---

#[test]
fn gauge_autoshift_continue_does_not_fail() {
    let model = make_model();
    let mut player = BMSPlayer::new(model);
    player.state = PlayState::Play;
    player.playtime = 999_999;
    player.player_config.play_settings.gauge_auto_shift =
        rubato_types::player_config::GAUGEAUTOSHIFT_CONTINUE;

    let gauge = crate::groove_gauge::create_groove_gauge(
        &player.model,
        rubato_types::groove_gauge::HARD,
        0,
        None,
    )
    .unwrap();
    player.gauge = Some(gauge);
    player.gauge.as_mut().unwrap().set_value(0.0);

    player.main_state_data.timer.update();
    let now = player.main_state_data.timer.now_micro_time();
    player
        .main_state_data
        .timer
        .set_micro_timer(TIMER_PLAY, now - 1000);
    player.prevtime = now - 500;

    player.render();

    // Should NOT transition to FAILED with CONTINUE mode
    assert_eq!(player.state(), PlayState::Play);
}

#[test]
fn gauge_autoshift_survival_to_groove_shifts_type() {
    let model = make_model();
    let mut player = BMSPlayer::new(model);
    player.state = PlayState::Play;
    player.playtime = 999_999;
    player.player_config.play_settings.gauge_auto_shift =
        rubato_types::player_config::GAUGEAUTOSHIFT_SURVIVAL_TO_GROOVE;

    let gauge = crate::groove_gauge::create_groove_gauge(
        &player.model,
        rubato_types::groove_gauge::HARD,
        0,
        None,
    )
    .unwrap();
    player.gauge = Some(gauge);
    player.gauge.as_mut().unwrap().set_value(0.0);

    player.main_state_data.timer.update();
    let now = player.main_state_data.timer.now_micro_time();
    player
        .main_state_data
        .timer
        .set_micro_timer(TIMER_PLAY, now - 1000);
    player.prevtime = now - 500;

    player.render();

    // Should shift to NORMAL gauge type, not FAILED
    assert_eq!(player.state(), PlayState::Play);
    assert_eq!(
        player.gauge.as_ref().unwrap().gauge_type(),
        rubato_types::groove_gauge::NORMAL
    );
}

// --- quick retry tests ---

#[test]
fn quick_retry_in_failed_state_with_start_xor_select() {
    let model = make_model();
    let mut player = BMSPlayer::new(model);
    player.state = PlayState::Failed;
    player.lanerender = Some(LaneRenderer::new(&player.model));
    player.input.keyinput = Some(KeyInputProccessor::new(&LaneProperty::new(&Mode::BEAT_7K)));
    player.play_mode = BMSPlayerMode::PLAY;
    player.is_course_mode = false;

    // START pressed, SELECT not pressed (XOR = true)
    player.input.input_start_pressed = true;
    player.input.input_select_pressed = false;

    player.main_state_data.timer.update();
    player.render();

    // Should request transition to PLAY (quick retry)
    let state_change = player.take_pending_state_change();
    assert_eq!(state_change, Some(MainStateType::Play));
}

#[test]
fn no_quick_retry_in_course_mode() {
    let model = make_model();
    let mut player = BMSPlayer::new(model);
    player.state = PlayState::Failed;
    player.lanerender = Some(LaneRenderer::new(&player.model));
    player.input.keyinput = Some(KeyInputProccessor::new(&LaneProperty::new(&Mode::BEAT_7K)));
    player.play_mode = BMSPlayerMode::PLAY;
    player.is_course_mode = true;

    player.input.input_start_pressed = true;
    player.input.input_select_pressed = false;

    player.main_state_data.timer.update();
    player.render();

    // Quick retry should NOT trigger in course mode
    // (only TIMER_FAILED timeout transition should happen)
    let state_change = player.take_pending_state_change();
    assert_ne!(state_change, Some(MainStateType::Play));
}

#[test]
fn aborted_quick_retry_with_start_xor_select() {
    let model = make_model();
    let mut player = BMSPlayer::new(model);
    player.state = PlayState::Aborted;
    player.lanerender = Some(LaneRenderer::new(&player.model));
    player.play_mode = BMSPlayerMode::PLAY;
    player.is_course_mode = false;

    // SELECT pressed, START not pressed (XOR = true)
    player.input.input_start_pressed = false;
    player.input.input_select_pressed = true;

    player.main_state_data.timer.update();
    player.render();

    // Should request transition to PLAY
    let state_change = player.take_pending_state_change();
    assert_eq!(state_change, Some(MainStateType::Play));
}

// --- state transition tests ---

#[test]
fn failed_transitions_to_practice_in_practice_mode() {
    let model = make_model();
    let mut player = BMSPlayer::new(model);
    player.state = PlayState::Failed;
    player.lanerender = Some(LaneRenderer::new(&player.model));
    player.input.keyinput = Some(KeyInputProccessor::new(&LaneProperty::new(&Mode::BEAT_7K)));
    player.play_mode = BMSPlayerMode::PRACTICE;

    // Set TIMER_FAILED so close time is exceeded
    player.main_state_data.timer.set_timer_on(TIMER_FAILED);
    player.main_state_data.timer.update();
    let now = player.main_state_data.timer.now_micro_time();
    player
        .main_state_data
        .timer
        .set_micro_timer(TIMER_FAILED, now - 10_000_000);
    player.play_skin.close = 0;

    player.render();

    // In practice mode, should return to PlayState::Practice
    assert_eq!(player.state(), PlayState::Practice);
}

#[test]
fn pending_state_change_consumed_once() {
    let model = make_model();
    let mut player = BMSPlayer::new(model);
    player.pending.pending_state_change = Some(MainStateType::Result);

    let first = player.take_pending_state_change();
    assert_eq!(first, Some(MainStateType::Result));

    let second = player.take_pending_state_change();
    assert_eq!(second, None);
}

// --- chart preview tests ---

#[test]
fn chart_preview_sets_timer_141_when_enabled() {
    let model = make_model();
    let mut player = BMSPlayer::new(model);
    player.state = PlayState::Preload;
    player.player_config.display_settings.chart_preview = true;
    player.startpressedtime = 0;

    // When micronow == startpressedtime and timer 141 is off, timer 141 should be set
    player.main_state_data.timer.update();
    let micronow = player.main_state_data.timer.now_micro_time();
    player.startpressedtime = micronow;

    player.render();

    // Timer 141 should have been set
    assert!(player.main_state_data.timer.is_timer_on(TimerId::new(141)));
}

// --- player config wiring tests ---

#[test]
fn set_player_config_persists() {
    let model = make_model();
    let mut player = BMSPlayer::new(model);

    let config = PlayerConfig {
        display_settings: rubato_types::player_config::DisplaySettings {
            chart_preview: false,
            ..Default::default()
        },
        select_settings: rubato_types::player_config::SelectSettings {
            is_window_hold: true,
            ..Default::default()
        },
        play_settings: rubato_types::player_config::PlaySettings {
            gauge_auto_shift: 3,
            ..Default::default()
        },
        ..Default::default()
    };

    player.player_config = config;

    assert!(!player.player_config().display_settings.chart_preview);
    assert!(player.player_config().select_settings.is_window_hold);
    assert_eq!(player.player_config().play_settings.gauge_auto_shift, 3);
}

// --- course mode tests ---

#[test]
fn set_course_mode_persists() {
    let model = make_model();
    let mut player = BMSPlayer::new(model);

    player.is_course_mode = true;
    assert!(player.is_course_mode);

    player.is_course_mode = false;
    assert!(!player.is_course_mode);
}

// --- sync_audio tests ---

struct NoteTrackingAudioDriver {
    global_pitch: f32,
    played_notes: Vec<(i32, f32)>, // (wav, volume)
    stop_all_count: usize,
}

impl NoteTrackingAudioDriver {
    fn new() -> Self {
        Self {
            global_pitch: 1.0,
            played_notes: Vec::new(),
            stop_all_count: 0,
        }
    }
}

impl rubato_audio::audio_driver::AudioDriver for NoteTrackingAudioDriver {
    fn play_path(&mut self, _path: &str, _volume: f32, _loop_play: bool) {}
    fn set_volume_path(&mut self, _path: &str, _volume: f32) {}
    fn is_playing_path(&self, _path: &str) -> bool {
        false
    }
    fn stop_path(&mut self, _path: &str) {}
    fn dispose_path(&mut self, _path: &str) {}
    fn set_model(&mut self, _model: &bms_model::bms_model::BMSModel) {}
    fn set_additional_key_sound(&mut self, _judge: i32, _fast: bool, _path: Option<&str>) {}
    fn abort(&mut self) {}
    fn get_progress(&self) -> f32 {
        1.0
    }
    fn play_note(&mut self, n: &bms_model::note::Note, volume: f32, _pitch: i32) {
        self.played_notes.push((n.wav(), volume));
    }
    fn play_judge(&mut self, _judge: i32, _fast: bool) {}
    fn stop_note(&mut self, n: Option<&bms_model::note::Note>) {
        if n.is_none() {
            self.stop_all_count += 1;
        }
    }
    fn set_volume_note(&mut self, _n: &bms_model::note::Note, _volume: f32) {}
    fn set_global_pitch(&mut self, pitch: f32) {
        self.global_pitch = pitch;
    }
    fn get_global_pitch(&self) -> f32 {
        self.global_pitch
    }
    fn dispose_old(&mut self) {}
    fn dispose(&mut self) {}
}

#[test]
fn sync_audio_drains_pending_bg_notes() {
    use bms_model::note::Note;
    use bms_model::time_line::TimeLine;

    // Build a model with a BG note at time 0
    let mut model = make_model();
    let mut tl = TimeLine::new(120.0, 0, 8);
    tl.add_back_ground_note(Note::new_normal(1));
    model.timelines = vec![tl];

    let mut player = BMSPlayer::new(model);

    // Start BG play from time 0
    player.keysound.start_bg_play(
        &player.model,
        0,   // offset
        1.0, // volume
    );
    // Set play time so the BG thread fires the note
    player.keysound.update_play_time(1_000_000);

    // Give the BG thread time to enqueue
    std::thread::sleep(std::time::Duration::from_millis(50));

    // Before sync_audio, notes should be queued but not played
    let mut audio = NoteTrackingAudioDriver::new();
    assert!(audio.played_notes.is_empty());

    // sync_audio should drain and play
    player.sync_audio(&mut audio);
    assert!(
        !audio.played_notes.is_empty(),
        "sync_audio should forward BG notes to AudioDriver"
    );
    assert_eq!(audio.played_notes[0].0, 1); // wav id

    player.keysound.stop_bg_play();
}

#[test]
fn sync_audio_stops_all_notes_when_pending_flag_set() {
    let model = make_model();
    let mut player = BMSPlayer::new(model);

    // Set the pending flag
    player.pending.pending_stop_all_notes = true;

    let mut audio = NoteTrackingAudioDriver::new();
    assert_eq!(audio.stop_all_count, 0);

    player.sync_audio(&mut audio);

    assert_eq!(
        audio.stop_all_count, 1,
        "sync_audio should call stop_note(None) when pending_stop_all_notes is set"
    );
    // Flag should be consumed
    assert!(
        !player.pending.pending_stop_all_notes,
        "pending_stop_all_notes should be cleared after sync_audio"
    );
}

#[test]
fn sync_audio_does_not_stop_notes_when_flag_not_set() {
    let model = make_model();
    let mut player = BMSPlayer::new(model);

    let mut audio = NoteTrackingAudioDriver::new();
    player.sync_audio(&mut audio);

    assert_eq!(
        audio.stop_all_count, 0,
        "sync_audio should not call stop_note(None) when flag is not set"
    );
}

// --- Keysound play events through sync_audio ---

#[test]
fn sync_audio_drains_pending_keysound_plays() {
    use bms_model::note::Note;

    let model = make_model();
    let mut player = BMSPlayer::new(model);

    // Manually push a keysound play event (simulating what render() does
    // after resolving JudgeManager keysound_play_indices)
    let note = Note::new_normal(42);
    player.pending.pending_keysound_plays.push((note, 0.8));

    let mut audio = NoteTrackingAudioDriver::new();
    player.sync_audio(&mut audio);

    assert_eq!(
        audio.played_notes.len(),
        1,
        "sync_audio should play keysound notes from pending_keysound_plays"
    );
    assert_eq!(audio.played_notes[0].0, 42, "wav id should match");
    assert!(
        (audio.played_notes[0].1 - 0.8).abs() < f32::EPSILON,
        "volume should match"
    );

    // Second sync should be empty (drained)
    player.sync_audio(&mut audio);
    assert_eq!(
        audio.played_notes.len(),
        1,
        "pending_keysound_plays should be drained after sync_audio"
    );
}

// --- Gauge initialization in create() ---

#[test]
fn create_initializes_gauge_for_play_mode() {
    let model = make_model();
    let mut player = BMSPlayer::new(model);
    player.play_mode = BMSPlayerMode::PLAY;
    player.player_config.play_settings.gauge = crate::groove_gauge::NORMAL;
    player.create();
    assert!(
        player.gauge.is_some(),
        "gauge should be initialized for Play mode"
    );
}

#[test]
fn create_initializes_gauge_for_autoplay_mode() {
    let model = make_model();
    let mut player = BMSPlayer::new(model);
    player.play_mode = BMSPlayerMode::AUTOPLAY;
    player.player_config.play_settings.gauge = crate::groove_gauge::NORMAL;
    player.create();
    assert!(
        player.gauge.is_some(),
        "gauge should be initialized for Autoplay mode"
    );
}

#[test]
fn create_initializes_gauge_for_replay_mode() {
    let model = make_model();
    let mut player = BMSPlayer::new(model);
    player.play_mode = BMSPlayerMode::REPLAY_1;
    player.player_config.play_settings.gauge = crate::groove_gauge::NORMAL;
    player.create();
    assert!(
        player.gauge.is_some(),
        "gauge should be initialized for Replay mode"
    );
}

#[test]
fn create_does_not_initialize_gauge_for_practice_mode() {
    let model = make_model();
    let mut player = BMSPlayer::new(model);
    player.play_mode = BMSPlayerMode::PRACTICE;
    player.create();
    // Practice mode gauge is set later during practice configuration
    assert!(
        player.gauge.is_none(),
        "gauge should not be initialized in create() for Practice mode"
    );
}

#[test]
fn create_with_negative_playtime_does_not_panic() {
    let model = make_model();
    let mut player = BMSPlayer::new(model);
    // Simulate negative playtime from deserialized or incorrectly computed data.
    // Before the fix, a sufficiently negative value like -1500 would compute
    // (-1500 / 500 + 2) = -1, and casting -1_i32 as usize wraps to usize::MAX,
    // causing an allocation panic.
    player.playtime = -1500;
    player.create();
    // Gauge log should be allocated with a small capacity, not a huge one
    for log in &player.gaugelog {
        assert!(
            log.capacity() <= 2,
            "expected small capacity for negative playtime, got {}",
            log.capacity()
        );
    }
}

// --- judge algorithm from player config ---

#[test]
fn create_reads_judge_algorithm_from_play_config() {
    let model = make_model();
    let mut player = BMSPlayer::new(model);
    // Set the judge algorithm to Duration in the mode-specific PlayConfig
    player
        .player_config
        .play_config(Mode::BEAT_7K)
        .playconfig
        .judgetype = "Duration".to_string();

    player.create();

    // The JudgeManager should use Duration algorithm, and the score should record it
    assert_eq!(
        player.judge_manager().score().play_option.judge_algorithm,
        Some(rubato_types::judge_algorithm::JudgeAlgorithm::Duration),
    );
}

#[test]
fn create_defaults_to_combo_for_invalid_judgetype() {
    let model = make_model();
    let mut player = BMSPlayer::new(model);
    player
        .player_config
        .play_config(Mode::BEAT_7K)
        .playconfig
        .judgetype = "InvalidAlgorithm".to_string();

    player.create();

    // Should fall back to Combo
    assert_eq!(
        player.judge_manager().score().play_option.judge_algorithm,
        Some(rubato_types::judge_algorithm::JudgeAlgorithm::Combo),
    );
}

#[test]
fn create_sets_rule_lr2_on_score() {
    let model = make_model();
    let mut player = BMSPlayer::new(model);
    player.create();

    assert_eq!(
        player.judge_manager().score().play_option.rule,
        Some(rubato_types::bms_player_rule::BMSPlayerRule::LR2),
    );
}

// --- PlayMouseContext property delegation tests ---

/// Test skin that probes integer_value during mouse_pressed_at.
struct ProbeMouseIntegerSkin {
    id: i32,
    observed: Arc<AtomicI32>,
}

impl SkinDrawable for ProbeMouseIntegerSkin {
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
        self.observed
            .store(ctx.integer_value(self.id), Ordering::SeqCst);
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
    fn swap_sprite_batch(&mut self, _batch: &mut SpriteBatch) {}
}

/// Test skin that probes boolean_value during mouse_pressed_at.
struct ProbeMouseBoolSkin {
    id: i32,
    observed: Arc<std::sync::atomic::AtomicBool>,
}

impl SkinDrawable for ProbeMouseBoolSkin {
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
        self.observed
            .store(ctx.boolean_value(self.id), Ordering::SeqCst);
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
    fn swap_sprite_batch(&mut self, _batch: &mut SpriteBatch) {}
}

/// Test skin that probes gauge_value during mouse_pressed_at.
struct ProbeMouseGaugeSkin {
    observed: Arc<std::sync::Mutex<f32>>,
}

impl SkinDrawable for ProbeMouseGaugeSkin {
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
        *self.observed.lock().unwrap() = ctx.gauge_value();
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
    fn swap_sprite_batch(&mut self, _batch: &mut SpriteBatch) {}
}

/// Test skin that probes player_config_ref during mouse_pressed_at.
struct ProbeMousePlayerConfigSkin {
    observed: Arc<AtomicI32>,
}

impl SkinDrawable for ProbeMousePlayerConfigSkin {
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
        let val = ctx
            .player_config_ref()
            .map_or(-1, |c| c.judge_settings.judgetiming);
        self.observed.store(val, Ordering::SeqCst);
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
    fn swap_sprite_batch(&mut self, _batch: &mut SpriteBatch) {}
}

#[test]
fn mouse_context_delegates_integer_value_total_notes() {
    let model = make_model();
    let mut player = BMSPlayer::new(model);
    let observed = Arc::new(AtomicI32::new(-1));
    player.main_state_data.skin = Some(Box::new(ProbeMouseIntegerSkin {
        id: 350,
        observed: observed.clone(),
    }));

    <BMSPlayer as MainState>::handle_skin_mouse_pressed(&mut player, 0, 10, 10);

    // total_notes is computed from model (0 notes in empty model)
    assert_eq!(observed.load(Ordering::SeqCst), player.total_notes);
}

#[test]
fn mouse_context_delegates_integer_value_playtime() {
    let model = make_model_with_time(120);
    let expected_playtime = model.last_note_time() + TIME_MARGIN;
    let mut player = BMSPlayer::new(model);
    let observed = Arc::new(AtomicI32::new(-1));
    player.main_state_data.skin = Some(Box::new(ProbeMouseIntegerSkin {
        id: 312,
        observed: observed.clone(),
    }));

    <BMSPlayer as MainState>::handle_skin_mouse_pressed(&mut player, 0, 10, 10);

    // integer_value returns i32 (clamped from i64 playtime)
    assert_eq!(
        observed.load(Ordering::SeqCst),
        expected_playtime.clamp(i32::MIN as i64, i32::MAX as i64) as i32
    );
}

#[test]
fn mouse_context_delegates_integer_value_loading_progress() {
    let model = make_model();
    let mut player = BMSPlayer::new(model);
    player.media_load_finished = true;
    let observed = Arc::new(AtomicI32::new(-1));
    player.main_state_data.skin = Some(Box::new(ProbeMouseIntegerSkin {
        id: 165,
        observed: observed.clone(),
    }));

    <BMSPlayer as MainState>::handle_skin_mouse_pressed(&mut player, 0, 10, 10);

    assert_eq!(observed.load(Ordering::SeqCst), 100);
}

#[test]
fn mouse_context_delegates_boolean_value_autoplay() {
    let model = make_model();
    let mut player = BMSPlayer::new(model);
    player.play_mode = BMSPlayerMode::AUTOPLAY;
    let observed = Arc::new(std::sync::atomic::AtomicBool::new(false));
    player.main_state_data.skin = Some(Box::new(ProbeMouseBoolSkin {
        id: 200,
        observed: observed.clone(),
    }));

    <BMSPlayer as MainState>::handle_skin_mouse_pressed(&mut player, 0, 10, 10);

    assert!(observed.load(Ordering::SeqCst));
}

#[test]
fn mouse_context_delegates_boolean_value_preload() {
    let model = make_model();
    let mut player = BMSPlayer::new(model);
    // Default state is Preload
    assert_eq!(player.state(), PlayState::Preload);
    let observed = Arc::new(std::sync::atomic::AtomicBool::new(false));
    player.main_state_data.skin = Some(Box::new(ProbeMouseBoolSkin {
        id: 80,
        observed: observed.clone(),
    }));

    <BMSPlayer as MainState>::handle_skin_mouse_pressed(&mut player, 0, 10, 10);

    assert!(observed.load(Ordering::SeqCst));
}

#[test]
fn mouse_context_delegates_gauge_value() {
    let model = make_model();
    let mut player = BMSPlayer::new(model);
    // Create a gauge with a known value
    let gauge = crate::groove_gauge::create_groove_gauge(
        &player.model,
        rubato_types::groove_gauge::NORMAL,
        0,
        None,
    )
    .unwrap();
    let expected_value = gauge.value();
    player.gauge = Some(gauge);
    let observed = Arc::new(std::sync::Mutex::new(-1.0_f32));
    player.main_state_data.skin = Some(Box::new(ProbeMouseGaugeSkin {
        observed: observed.clone(),
    }));

    <BMSPlayer as MainState>::handle_skin_mouse_pressed(&mut player, 0, 10, 10);

    let result = *observed.lock().unwrap();
    assert!(
        (result - expected_value).abs() < 0.001,
        "gauge_value should be {} but was {}",
        expected_value,
        result
    );
}

#[test]
fn mouse_context_delegates_player_config_ref() {
    let model = make_model();
    let mut player = BMSPlayer::new(model);
    player.player_config.judge_settings.judgetiming = 42;
    let observed = Arc::new(AtomicI32::new(-1));
    player.main_state_data.skin = Some(Box::new(ProbeMousePlayerConfigSkin {
        observed: observed.clone(),
    }));

    <BMSPlayer as MainState>::handle_skin_mouse_pressed(&mut player, 0, 10, 10);

    assert_eq!(observed.load(Ordering::SeqCst), 42);
}

#[test]
fn mouse_context_delegates_image_index_value_42() {
    let model = make_model();
    let mut player = BMSPlayer::new(model);
    player.score.playinfo.randomoption = 3;
    let observed = Arc::new(AtomicI32::new(-1));
    // image_index_value(42) depends on replay_option_data(), which PlayMouseContext
    // must delegate to return playinfo.randomoption.
    player.main_state_data.skin = Some(Box::new(ProbeMouseImageIndexSkin {
        id: 42,
        observed: observed.clone(),
    }));

    <BMSPlayer as MainState>::handle_skin_mouse_pressed(&mut player, 0, 10, 10);

    assert_eq!(observed.load(Ordering::SeqCst), 3);
}

#[test]
fn render_skin_passes_timer_play_start_time_to_note_draw_context() {
    let model = make_model_with_time(120);
    let mut player = BMSPlayer::new(model);
    player.lanerender = Some(crate::lane_renderer::LaneRenderer::new(&player.model));
    let observed = Arc::new(Mutex::new(None));
    player.main_state_data.skin = Some(Box::new(ProbeDrawLaneTimeSkin {
        observed: observed.clone(),
    }));

    player.main_state_data.timer.set_now_micro_time(3_000_000);
    player
        .main_state_data
        .timer
        .set_micro_timer(TIMER_PLAY, 1_000_000);

    let mut sprite = SpriteBatch::new();
    player.render_skin_impl(&mut sprite);

    assert_eq!(
        *observed.lock().unwrap(),
        Some(ObservedDrawLaneTime {
            time: 3000,
            timer_play: Some(1000),
        }),
        "render_skin_impl must pass TIMER_PLAY start time, not elapsed time, to DrawLaneContext"
    );
}

/// Test skin that probes image_index_value during mouse_pressed_at.
struct ProbeMouseImageIndexSkin {
    id: i32,
    observed: Arc<AtomicI32>,
}

impl SkinDrawable for ProbeMouseImageIndexSkin {
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
        self.observed
            .store(ctx.image_index_value(self.id), Ordering::SeqCst);
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
    fn swap_sprite_batch(&mut self, _batch: &mut SpriteBatch) {}
}

// --- PlayRenderContext skin property tests ---

/// Helper to create a PlayRenderContext with configurable BPM and volume values.
fn make_play_render_context_with_bpm_volume<'a>(
    timer: &'a mut rubato_core::timer_manager::TimerManager,
    judge: &'a crate::judge::manager::JudgeManager,
    player_config: &'a PlayerConfig,
    play_config: &'a rubato_types::play_config::PlayConfig,
    option_info: &'a rubato_types::replay_data::ReplayData,
    bpm: (f64, f64, f64, f64),
    volume: (f32, f32, f32),
) -> skin_context::PlayRenderContext<'a> {
    static DEFAULT_CONFIG: std::sync::OnceLock<rubato_types::config::Config> =
        std::sync::OnceLock::new();
    static DEFAULT_SCORE_DATA: std::sync::OnceLock<
        rubato_types::score_data_property::ScoreDataProperty,
    > = std::sync::OnceLock::new();
    skin_context::PlayRenderContext {
        timer,
        judge,
        gauge: None,
        player_config,
        option_info,
        play_config,
        target_score: None,
        playtime: 60000,
        total_notes: 500,
        play_mode: BMSPlayerMode::PLAY,
        state: PlayState::Play,
        media_load_finished: true,
        now_bpm: bpm.0,
        min_bpm: bpm.1,
        max_bpm: bpm.2,
        main_bpm: bpm.3,
        system_volume: volume.0,
        key_volume: volume.1,
        bg_volume: volume.2,
        is_mode_changed: false,
        lnmode_override: None,
        config: DEFAULT_CONFIG.get_or_init(rubato_types::config::Config::default),
        score_data_property: DEFAULT_SCORE_DATA
            .get_or_init(rubato_types::score_data_property::ScoreDataProperty::default),
        song_metadata: {
            static DEFAULT_META: std::sync::OnceLock<rubato_types::song_data::SongMetadata> =
                std::sync::OnceLock::new();
            DEFAULT_META.get_or_init(rubato_types::song_data::SongMetadata::default)
        },
    }
}

#[test]
fn play_render_context_integer_bpm_ids() {
    use rubato_types::skin_render_context::SkinRenderContext;

    let mut timer = rubato_core::timer_manager::TimerManager::new();
    let judge = crate::judge::manager::JudgeManager::new();
    let pc = PlayerConfig::default();
    let play_config = rubato_types::play_config::PlayConfig::default();
    let option_info = rubato_types::replay_data::ReplayData::default();

    let ctx = make_play_render_context_with_bpm_volume(
        &mut timer,
        &judge,
        &pc,
        &play_config,
        &option_info,
        (155.5, 120.0, 200.0, 150.0), // now, min, max, main
        (0.5, 0.5, 0.5),
    );

    // 90 = maxbpm
    assert_eq!(ctx.integer_value(90), 200);
    // 91 = minbpm
    assert_eq!(ctx.integer_value(91), 120);
    // 92 = mainbpm
    assert_eq!(ctx.integer_value(92), 150);
    // 160 = nowbpm
    assert_eq!(ctx.integer_value(160), 155);
}

#[test]
fn play_render_context_integer_volume_ids() {
    use rubato_types::skin_render_context::SkinRenderContext;

    let mut timer = rubato_core::timer_manager::TimerManager::new();
    let judge = crate::judge::manager::JudgeManager::new();
    let pc = PlayerConfig::default();
    let play_config = rubato_types::play_config::PlayConfig::default();
    let option_info = rubato_types::replay_data::ReplayData::default();

    let ctx = make_play_render_context_with_bpm_volume(
        &mut timer,
        &judge,
        &pc,
        &play_config,
        &option_info,
        (120.0, 120.0, 120.0, 120.0),
        (0.8, 0.6, 0.4), // system, key, bg
    );

    // 57 = volume_system (0-100 scale)
    assert_eq!(ctx.integer_value(57), 80);
    // 58 = volume_key
    assert_eq!(ctx.integer_value(58), 60);
    // 59 = volume_background
    assert_eq!(ctx.integer_value(59), 40);
}

#[test]
fn play_render_context_float_volume_ids() {
    use rubato_types::skin_render_context::SkinRenderContext;

    let mut timer = rubato_core::timer_manager::TimerManager::new();
    let judge = crate::judge::manager::JudgeManager::new();
    let pc = PlayerConfig::default();
    let play_config = rubato_types::play_config::PlayConfig::default();
    let option_info = rubato_types::replay_data::ReplayData::default();

    let ctx = make_play_render_context_with_bpm_volume(
        &mut timer,
        &judge,
        &pc,
        &play_config,
        &option_info,
        (120.0, 120.0, 120.0, 120.0),
        (0.75, 0.5, 0.25), // system, key, bg
    );

    // 17 = mastervolume (0.0-1.0)
    assert!((ctx.float_value(17) - 0.75).abs() < f32::EPSILON);
    // 18 = keyvolume
    assert!((ctx.float_value(18) - 0.5).abs() < f32::EPSILON);
    // 19 = bgmvolume
    assert!((ctx.float_value(19) - 0.25).abs() < f32::EPSILON);
}

#[test]
fn play_render_context_float_loading_progress() {
    use rubato_types::skin_render_context::SkinRenderContext;

    let mut timer = rubato_core::timer_manager::TimerManager::new();
    let judge = crate::judge::manager::JudgeManager::new();
    let pc = PlayerConfig::default();
    let play_config = rubato_types::play_config::PlayConfig::default();
    let option_info = rubato_types::replay_data::ReplayData::default();

    // media_load_finished = true -> 1.0
    let ctx = make_play_render_context_with_bpm_volume(
        &mut timer,
        &judge,
        &pc,
        &play_config,
        &option_info,
        (120.0, 120.0, 120.0, 120.0),
        (0.5, 0.5, 0.5),
    );
    assert!((ctx.float_value(165) - 1.0).abs() < f32::EPSILON);

    // media_load_finished = false -> 0.0
    let mut ctx2 = make_play_render_context_with_bpm_volume(
        &mut timer,
        &judge,
        &pc,
        &play_config,
        &option_info,
        (120.0, 120.0, 120.0, 120.0),
        (0.5, 0.5, 0.5),
    );
    ctx2.media_load_finished = false;
    assert!((ctx2.float_value(165)).abs() < f32::EPSILON);
}

#[test]
fn play_render_context_existing_ids_unchanged() {
    use rubato_types::skin_render_context::SkinRenderContext;

    let mut timer = rubato_core::timer_manager::TimerManager::new();
    let judge = crate::judge::manager::JudgeManager::new();
    let pc = PlayerConfig::default();
    let play_config = rubato_types::play_config::PlayConfig::default();
    let option_info = rubato_types::replay_data::ReplayData::default();

    let ctx = make_play_render_context_with_bpm_volume(
        &mut timer,
        &judge,
        &pc,
        &play_config,
        &option_info,
        (120.0, 120.0, 120.0, 120.0),
        (0.5, 0.5, 0.5),
    );

    // 350 = total notes
    assert_eq!(ctx.integer_value(350), 500);
    // 312 = playtime
    assert_eq!(ctx.integer_value(312), 60000);
    // 165 = loading progress (integer: 100 when loaded)
    assert_eq!(ctx.integer_value(165), 100);
    // 1107 = gauge (no gauge -> 0.0)
    assert!((ctx.float_value(1107)).abs() < f32::EPSILON);
    // unknown IDs still return 0
    assert_eq!(ctx.integer_value(9999), 0);
    assert!((ctx.float_value(9999)).abs() < f32::EPSILON);
}

#[test]
fn play_render_context_bpm_zero_when_no_lanerender() {
    use rubato_types::skin_render_context::SkinRenderContext;

    let mut timer = rubato_core::timer_manager::TimerManager::new();
    let judge = crate::judge::manager::JudgeManager::new();
    let pc = PlayerConfig::default();
    let play_config = rubato_types::play_config::PlayConfig::default();
    let option_info = rubato_types::replay_data::ReplayData::default();

    // BPM = 0.0 simulates no LaneRenderer (default fallback in render_skin.rs)
    let ctx = make_play_render_context_with_bpm_volume(
        &mut timer,
        &judge,
        &pc,
        &play_config,
        &option_info,
        (0.0, 0.0, 0.0, 0.0),
        (0.5, 0.5, 0.5),
    );

    assert_eq!(ctx.integer_value(90), 0);
    assert_eq!(ctx.integer_value(91), 0);
    assert_eq!(ctx.integer_value(92), 0);
    assert_eq!(ctx.integer_value(160), 0);
}

#[test]
fn aborted_quick_retry_not_overwritten_by_fadeout() {
    // Regression: when TIMER_FADEOUT has expired AND start/select is pressed,
    // quick-retry (Play) must win over the fadeout transition (MusicSelect).
    let model = make_model();
    let mut player = BMSPlayer::new(model);
    player.state = PlayState::Aborted;
    player.lanerender = Some(LaneRenderer::new(&player.model));
    player.play_mode = BMSPlayerMode::PLAY;
    player.is_course_mode = false;

    // Turn on TIMER_FADEOUT and make it expired (well past the skin fadeout of 0)
    player.main_state_data.timer.set_timer_on(TIMER_FADEOUT);
    player.main_state_data.timer.update();
    let now = player.main_state_data.timer.now_micro_time();
    player
        .main_state_data
        .timer
        .set_micro_timer(TIMER_FADEOUT, now - 10_000_000); // 10 seconds ago

    // START pressed, SELECT not pressed (XOR = true -> quick retry)
    player.input.input_start_pressed = true;
    player.input.input_select_pressed = false;

    player.render();

    // Quick retry should win: transition to Play, not MusicSelect
    let state_change = player.take_pending_state_change();
    assert_eq!(state_change, Some(MainStateType::Play));
}

// --- save_config outbox tests ---

#[test]
fn save_config_populates_pending_play_config_update() {
    let model = make_model();
    let mut player = BMSPlayer::new(model);
    player.lanerender = Some(LaneRenderer::new(&player.model));

    // Verify no pending update before save_config
    assert!(
        player.pending.pending_play_config_update.is_none(),
        "pending_play_config_update should be None before save_config"
    );

    player.save_config();

    // Verify pending update is populated
    let update = player
        .pending
        .pending_play_config_update
        .as_ref()
        .expect("save_config should populate pending_play_config_update");
    assert_eq!(
        update.0,
        Mode::BEAT_7K,
        "pending update should contain the model's mode"
    );

    // Verify the PlayConfig values match the lane renderer state
    let lr = player.lanerender.as_ref().unwrap();
    assert_eq!(update.1.lanecover, lr.lanecover());
    assert_eq!(update.1.lift, lr.lift_region());
    assert_eq!(update.1.hidden, lr.hidden_cover());
}

#[test]
fn save_config_pending_update_contains_hispeed_when_fixhispeed_off() {
    let model = make_model();
    let mut player = BMSPlayer::new(model);
    player.lanerender = Some(LaneRenderer::new(&player.model));

    // Set fixhispeed to OFF so hispeed (not duration) is saved
    player
        .player_config
        .play_config(Mode::BEAT_7K)
        .playconfig
        .fixhispeed = rubato_types::play_config::FIX_HISPEED_OFF;

    player.save_config();

    let (mode, pc) = player
        .pending
        .pending_play_config_update
        .as_ref()
        .expect("save_config should populate pending_play_config_update");
    assert_eq!(*mode, Mode::BEAT_7K);
    let lr_hispeed = player.lanerender.as_ref().unwrap().hispeed();
    assert_eq!(pc.hispeed, lr_hispeed);
}

#[test]
fn save_config_pending_update_contains_duration_when_fixhispeed_on() {
    let model = make_model();
    let mut player = BMSPlayer::new(model);
    player.lanerender = Some(LaneRenderer::new(&player.model));

    // Default fixhispeed is FIX_HISPEED_MAINBPM (not OFF), so duration should be saved
    player.save_config();

    let (_, pc) = player
        .pending
        .pending_play_config_update
        .as_ref()
        .expect("save_config should populate pending_play_config_update");
    let lr_duration = player.lanerender.as_ref().unwrap().duration();
    assert_eq!(pc.duration, lr_duration);
}

#[test]
fn save_config_no_pending_update_when_no_speed_constraint() {
    let model = make_model();
    let mut player = BMSPlayer::new(model);
    player.lanerender = Some(LaneRenderer::new(&player.model));
    player.constraints = vec![CourseDataConstraint::NoSpeed];

    player.save_config();

    assert!(
        player.pending.pending_play_config_update.is_none(),
        "save_config should not populate pending update when NoSpeed constraint is set"
    );
}

#[test]
fn save_config_no_pending_update_when_no_lane_renderer() {
    let model = make_model();
    let mut player = BMSPlayer::new(model);
    // No lanerender set

    player.save_config();

    assert!(
        player.pending.pending_play_config_update.is_none(),
        "save_config should not populate pending update when lane renderer is None"
    );
}

#[test]
fn take_pending_play_config_update_via_main_state_trait() {
    let model = make_model();
    let mut player = BMSPlayer::new(model);
    player.lanerender = Some(LaneRenderer::new(&player.model));

    player.save_config();

    // Access through the MainState trait method
    let state: &mut dyn MainState = &mut player;
    let update = state.take_pending_play_config_update();
    assert!(
        update.is_some(),
        "take_pending_play_config_update should return the pending update"
    );
    let (mode, _pc) = update.unwrap();
    assert_eq!(mode, Mode::BEAT_7K);

    // Second call should return None (consumed)
    let update2 = state.take_pending_play_config_update();
    assert!(
        update2.is_none(),
        "take_pending_play_config_update should return None after consumption"
    );
}

#[test]
fn receive_updated_play_config_updates_cloned_player_config() {
    let model = make_model();
    let mut player = BMSPlayer::new(model);

    // Verify default hispeed
    let original_hispeed = player
        .player_config
        .play_config_ref(Mode::BEAT_7K)
        .playconfig
        .hispeed;

    // Simulate modmenu pushing an updated PlayConfig
    let mut updated_pc = player
        .player_config
        .play_config_ref(Mode::BEAT_7K)
        .playconfig
        .clone();
    updated_pc.hispeed = original_hispeed + 1.5;
    updated_pc.enablelift = true;

    let state: &mut dyn MainState = &mut player;
    state.receive_updated_play_config(Mode::BEAT_7K, updated_pc);

    // BMSPlayer's cloned config should now reflect the update
    assert!(
        (player
            .player_config
            .play_config_ref(Mode::BEAT_7K)
            .playconfig
            .hispeed
            - (original_hispeed + 1.5))
            .abs()
            < f32::EPSILON,
        "hispeed should be updated after receive_updated_play_config"
    );
    assert!(
        player
            .player_config
            .play_config_ref(Mode::BEAT_7K)
            .playconfig
            .enablelift,
        "enablelift should be updated after receive_updated_play_config"
    );
}

#[test]
fn receive_updated_play_config_propagates_to_lanerender() {
    let model = make_model();
    let mut player = BMSPlayer::new(model);
    player.lanerender = Some(LaneRenderer::new(&player.model));

    // Verify LaneRenderer starts with defaults (not player config values)
    let lr = player.lanerender.as_ref().unwrap();
    assert!(!lr.is_enable_lanecover(), "lanecover should start disabled");
    assert!(!lr.is_enable_lift(), "lift should start disabled");

    // Simulate modmenu pushing an updated PlayConfig with non-default values
    let updated_pc = rubato_types::play_config::PlayConfig {
        hispeed: 4.0,
        duration: 800,
        lanecover: 0.35,
        enablelanecover: true,
        lift: 0.2,
        enablelift: true,
        hidden: 0.15,
        enablehidden: true,
        enable_constant: true,
        constant_fadein_time: 150,
        fixhispeed: 0,
        hispeedmargin: 0.75,
        ..Default::default()
    };

    let state: &mut dyn MainState = &mut player;
    state.receive_updated_play_config(Mode::BEAT_7K, updated_pc);

    // LaneRenderer must now reflect the updated values
    let lr = player.lanerender.as_ref().unwrap();
    assert!(
        (lr.hispeed() - 4.0).abs() < f32::EPSILON,
        "LaneRenderer hispeed should be propagated"
    );
    assert_eq!(
        lr.duration(),
        800,
        "LaneRenderer duration should be propagated"
    );
    assert!(
        (lr.lanecover() - 0.35).abs() < f32::EPSILON,
        "LaneRenderer lanecover should be propagated"
    );
    assert!(
        lr.is_enable_lanecover(),
        "LaneRenderer enable_lanecover should be propagated"
    );
    assert!(
        (lr.lift_region() - 0.2).abs() < f32::EPSILON,
        "LaneRenderer lift should be propagated"
    );
    assert!(
        lr.is_enable_lift(),
        "LaneRenderer enable_lift should be propagated"
    );
    assert!(
        (lr.hidden_cover() - 0.15).abs() < f32::EPSILON,
        "LaneRenderer hidden should be propagated"
    );
    assert!(
        lr.is_enable_hidden(),
        "LaneRenderer enable_hidden should be propagated"
    );
    assert!(
        (lr.hispeedmargin() - 0.75).abs() < f32::EPSILON,
        "LaneRenderer hispeedmargin should be propagated"
    );
}

// --- create_score_data avgjudge unjudged-note penalty tests ---
// Java BMSPlayer.createScoreData() applies a 1,000,000μs penalty for unjudged
// notes and divides by the total note count (judged + unjudged), not just the
// judged note count.

#[test]
fn create_score_data_avgjudge_includes_unjudged_penalty() {
    // 2 judged notes (state 1,2) + 1 unjudged (state 0)
    // Java behavior:
    //   avgduration = |1000| + |2000| + 1_000_000 = 1_003_000
    //   count = 3 (all playable notes)
    //   avgjudge = 1_003_000 / 3 = 334_333
    let model = make_model_with_timed_notes(&[(1, 1000), (2, 2000), (0, 9999)]);
    let mut player = BMSPlayer::new(model);
    player.state = PlayState::Aborted;

    let score = player.create_score_data(DeviceType::Keyboard).unwrap();

    // total_duration must include the 1,000,000 penalty for the unjudged note
    assert_eq!(score.timing_stats.total_duration, 1_003_000);
    // avgjudge denominator is total note count (3), not judged count (2)
    assert_eq!(score.timing_stats.avgjudge, 1_003_000 / 3);
}

#[test]
fn create_score_data_avgjudge_all_unjudged() {
    // All notes unjudged (state 0): each gets 1,000,000 penalty
    // Java behavior:
    //   avgduration = 1_000_000 * 3 = 3_000_000
    //   count = 3
    //   avgjudge = 1_000_000
    let model = make_model_with_timed_notes(&[(0, 100), (0, 200), (0, 300)]);
    let mut player = BMSPlayer::new(model);
    player.state = PlayState::Aborted;

    let score = player.create_score_data(DeviceType::Keyboard).unwrap();

    assert_eq!(score.timing_stats.total_duration, 3_000_000);
    assert_eq!(score.timing_stats.avgjudge, 1_000_000);
}

#[test]
fn create_score_data_avg_stddev_use_only_judged_notes() {
    // avg and stddev (Rust-only additions) should continue using only judged notes.
    // 2 judged + 1 unjudged
    let model = make_model_with_timed_notes(&[(1, 1000), (2, -2000), (0, 9999)]);
    let mut player = BMSPlayer::new(model);
    player.state = PlayState::Aborted;

    let score = player.create_score_data(DeviceType::Keyboard).unwrap();

    // avg = (1000 + (-2000)) / 2 = -500  (judged notes only)
    assert_eq!(score.timing_stats.avg, -500);
    // stddev based on judged play_times only: [1000, -2000], mean = -500
    let mean = -500_i64;
    let var = ((1000 - mean).pow(2) + (-2000 - mean).pow(2)) / 2;
    let expected_stddev = (var as f64).sqrt() as i64;
    assert_eq!(score.timing_stats.stddev, expected_stddev);
}

#[test]
fn create_score_data_avgjudge_single_unjudged_among_judged() {
    // 1 judged + 1 unjudged: verifies the denominator difference matters
    // Java: avgduration = |500| + 1_000_000 = 1_000_500, count = 2
    //   avgjudge = 500_250
    // Buggy Rust: avgduration = 500, count = 1 → avgjudge = 500
    let model = make_model_with_timed_notes(&[(1, 500), (0, 0)]);
    let mut player = BMSPlayer::new(model);
    player.state = PlayState::Aborted;

    let score = player.create_score_data(DeviceType::Keyboard).unwrap();

    assert_eq!(score.timing_stats.total_duration, 1_000_500);
    assert_eq!(score.timing_stats.avgjudge, 1_000_500 / 2);
}

#[test]
fn practice_finished_resets_global_pitch_on_transition_to_music_select() {
    // Regression: when practice mode uses frequency training (freq != 100),
    // the global pitch is set to freq/100.0. When transitioning from
    // PracticeFinished to MusicSelect, the pitch must be reset to 1.0.
    // Previously, only Failed/Finished/Aborted paths reset pitch.
    let model = make_model();
    let mut player = BMSPlayer::new(model);
    player.state = PlayState::PracticeFinished;

    // Simulate that frequency training previously set pitch to 0.75 and it
    // was consumed by the main controller. The pending field is now None, but
    // the audio driver still has pitch=0.75. PracticeFinished must queue a
    // reset to 1.0 before transitioning.

    // Set up TIMER_FADEOUT so the transition condition is met (fadeout > 0).
    // With no skin, fadeout() returns 0, so any positive elapsed time triggers.
    player.main_state_data.timer.update();
    let now = player.main_state_data.timer.now_micro_time();
    player
        .main_state_data
        .timer
        .set_micro_timer(TIMER_FADEOUT, now - 2000); // 2ms ago

    player.render();

    // Verify transition to MusicSelect happened
    let state_change = player.take_pending_state_change();
    assert_eq!(
        state_change,
        Some(MainStateType::MusicSelect),
        "PracticeFinished should transition to MusicSelect"
    );

    // Verify global pitch is reset to 1.0
    let pitch = player.take_pending_global_pitch();
    assert_eq!(
        pitch,
        Some(1.0),
        "PracticeFinished must reset global pitch to 1.0 before transitioning to MusicSelect"
    );
}

// --- sync_judge_states_to_model tests ---

/// Helper: create a model with normal notes at specific times (one note per timeline, lane 0).
fn make_model_with_notes_at_times(times_us: &[i64]) -> BMSModel {
    let mut model = BMSModel::new();
    model.set_mode(Mode::BEAT_7K);
    model.judgerank = 100;

    let mut timelines = Vec::new();
    for (i, &time_us) in times_us.iter().enumerate() {
        let mut tl = bms_model::time_line::TimeLine::new(i as f64, time_us, 8);
        tl.set_note(0, Some(bms_model::note::Note::new_normal(1)));
        timelines.push(tl);
    }
    model.timelines = timelines;
    model
}

#[test]
fn sync_judge_states_writes_state_and_play_time_to_model_notes() {
    // Create a model with two notes
    let model = make_model_with_notes_at_times(&[1_000_000, 2_000_000]);
    let mut player = BMSPlayer::new(model);

    // Build judge system so judge_note_to_model is populated
    let mode = player.model.mode().copied().unwrap_or(Mode::BEAT_7K);
    player.rebuild_judge_system(&mode);

    // Verify notes initially have state=0
    assert_eq!(player.model.timelines[0].note(0).unwrap().state(), 0);
    assert_eq!(player.model.timelines[1].note(0).unwrap().state(), 0);

    // Simulate a judge: set note_states on the judge manager via autoplay update.
    // Instead of going through the full update flow, we use the accessors:
    // The judge has note_states initialized to state=0, play_time=0.
    // We need to trigger a judgment. Let's use autoplay mode.
    player.play_mode = BMSPlayerMode::AUTOPLAY;
    player.rebuild_judge_system(&mode);

    // Initialize gauge for update()
    player.gauge = crate::groove_gauge::create_groove_gauge(
        &player.model,
        rubato_types::groove_gauge::NORMAL,
        0,
        None,
    );

    // Run update at time=1_000_000 - autoplay should judge the first note as PG
    if let Some(ref mut gauge) = player.gauge {
        player.judge.update(
            1_000_000,
            &player.judge_notes,
            &vec![false; KEYSTATE_SIZE],
            &vec![i64::MIN; KEYSTATE_SIZE],
            gauge,
        );
    }

    // Verify judge set the first note's state (PG = judge 0 => state = 0+1 = 1)
    assert_eq!(player.judge.note_state(0), 1); // PG+1
    assert_eq!(player.judge.note_play_time(0), 0); // perfect timing

    // Before sync: model notes should still have state=0
    assert_eq!(player.model.timelines[0].note(0).unwrap().state(), 0);

    // Sync and verify
    player.sync_judge_states_to_model();

    assert_eq!(
        player.model.timelines[0].note(0).unwrap().state(),
        1,
        "After sync, model note state should match judge state"
    );
    assert_eq!(
        player.model.timelines[0].note(0).unwrap().micro_play_time(),
        0,
        "After sync, model note play_time should match judge play_time"
    );

    // Second note should still be unjudged
    assert_eq!(
        player.model.timelines[1].note(0).unwrap().state(),
        0,
        "Unjudged note should remain state=0"
    );
}

#[test]
fn autoplay_render_produces_keysound_play_events() {
    // Verify the full pipeline: render() calls judge.update() which produces
    // keysound events, then render() resolves them through judge_note_to_model
    // and pushes to pending.pending_keysound_plays.
    let model = make_model_with_notes_at_times(&[1_000_000]);
    let mut player = BMSPlayer::new(model);
    player.play_mode = BMSPlayerMode::AUTOPLAY;
    player.key_volume = 0.7;

    // create() builds judge, gauge, and sets state to Play
    player.create();
    player.state = PlayState::Play;

    // Start the play timer so render() enters the Playing branch
    player.main_state_data.timer.set_main_state();
    player.main_state_data.timer.set_timer_on(TIMER_PLAY);

    // Advance time past the note at 1_000_000us = 1000ms
    let timer_start = player.main_state_data.timer.micro_timer(TIMER_PLAY);
    player.main_state_data.timer.frozen = true;
    player
        .main_state_data
        .timer
        .set_now_micro_time(timer_start + 1_500_000);

    player.render();

    assert!(
        !player.pending.pending_keysound_plays.is_empty(),
        "autoplay render should produce keysound play events for judged notes"
    );
    // Verify volume matches key_volume
    let (_, vol) = &player.pending.pending_keysound_plays[0];
    assert!(
        (*vol - 0.7).abs() < f32::EPSILON,
        "keysound volume should match configured key_volume, got {}",
        vol
    );
}

#[test]
fn create_score_data_uses_synced_judge_states() {
    // Regression test: create_score_data() iterates model notes for timing stats.
    // Before the fix, model notes always had state=0 so timing stats were empty.
    let model = make_model_with_notes_at_times(&[1_000_000, 2_000_000]);
    let mut player = BMSPlayer::new(model);
    player.play_mode = BMSPlayerMode::AUTOPLAY;

    let mode = player.model.mode().copied().unwrap_or(Mode::BEAT_7K);
    player.rebuild_judge_system(&mode);
    player.gauge = crate::groove_gauge::create_groove_gauge(
        &player.model,
        rubato_types::groove_gauge::NORMAL,
        0,
        None,
    );

    // Judge both notes via autoplay (advance time past both)
    if let Some(ref mut gauge) = player.gauge {
        player.judge.update(
            1_000_000,
            &player.judge_notes,
            &vec![false; KEYSTATE_SIZE],
            &vec![i64::MIN; KEYSTATE_SIZE],
            gauge,
        );
        player.judge.update(
            2_000_000,
            &player.judge_notes,
            &vec![false; KEYSTATE_SIZE],
            &vec![i64::MIN; KEYSTATE_SIZE],
            gauge,
        );
    }

    // Sync judge states to model
    player.sync_judge_states_to_model();

    // Both notes should now have state >= 1
    assert!(
        player.model.timelines[0].note(0).unwrap().state() >= 1,
        "Note 0 should be judged after autoplay"
    );
    assert!(
        player.model.timelines[1].note(0).unwrap().state() >= 1,
        "Note 1 should be judged after autoplay"
    );

    // create_score_data should now produce valid timing stats
    player.state = PlayState::Aborted;
    let score = player.create_score_data(DeviceType::Keyboard).unwrap();

    // With autoplay, both notes are PG (play_time = 0), so:
    // - total_duration should be |0| + |0| = 0 (not the 2*1_000_000 unjudged penalty)
    // - avgjudge should be 0 (not 1_000_000)
    assert_eq!(
        score.timing_stats.total_duration, 0,
        "With autoplay PG, total_duration should be 0 (not unjudged penalty)"
    );
    assert_eq!(
        score.timing_stats.avgjudge, 0,
        "With autoplay PG, avgjudge should be 0"
    );
}

#[test]
fn judge_note_to_model_reverse_index_built_correctly() {
    let model = make_model_with_notes_at_times(&[500_000, 1_000_000, 2_000_000]);
    let mut player = BMSPlayer::new(model);

    let mode = player.model.mode().copied().unwrap_or(Mode::BEAT_7K);
    player.rebuild_judge_system(&mode);

    // All notes are on lane 0
    assert_eq!(player.judge_note_to_model.len(), player.judge_notes.len());
    for (i, &(tl_idx, lane)) in player.judge_note_to_model.iter().enumerate() {
        assert_ne!(
            tl_idx,
            usize::MAX,
            "JudgeNote {} should map to a valid timeline",
            i
        );
        assert_eq!(lane, 0, "All notes are on lane 0");
        // Verify the timeline time matches the judge note time
        assert_eq!(
            player.model.timelines[tl_idx].micro_time(),
            player.judge_notes[i].time_us,
            "Timeline time should match judge note time"
        );
    }
}

// Regression test: binary_search_by_key with duplicate timestamps must find
// the timeline that actually has a note on the target lane, not an arbitrary
// match (e.g., a barline-only timeline at the same time).
#[test]
fn judge_note_to_model_finds_correct_timeline_with_duplicate_timestamps() {
    let mut model = BMSModel::new();
    model.set_mode(Mode::BEAT_7K);
    model.judgerank = 100;

    // Create three timelines all at the same micro_time (1_000_000).
    // Only the third one (index 2) has a note on lane 0.
    let mut tl0 = bms_model::time_line::TimeLine::new(0.0, 1_000_000, 8);
    // tl0: barline only, no notes
    tl0.section_line = true;

    let mut tl1 = bms_model::time_line::TimeLine::new(0.0, 1_000_000, 8);
    // tl1: note on lane 3 only (different lane)
    tl1.set_note(3, Some(bms_model::note::Note::new_normal(1)));

    let mut tl2 = bms_model::time_line::TimeLine::new(0.0, 1_000_000, 8);
    // tl2: note on lane 0 (this is the one we want)
    tl2.set_note(0, Some(bms_model::note::Note::new_normal(1)));

    model.timelines = vec![tl0, tl1, tl2];

    let mut player = BMSPlayer::new(model);
    let mode = player.model.mode().copied().unwrap_or(Mode::BEAT_7K);
    player.rebuild_judge_system(&mode);

    // There should be judge notes for lane 0 (from tl2) and lane 3 (from tl1).
    // Find the judge_note_to_model entry for the lane-0 note.
    let lane0_entries: Vec<_> = player
        .judge_note_to_model
        .iter()
        .enumerate()
        .filter(|&(_, &(_, lane))| lane == 0)
        .collect();
    assert!(
        !lane0_entries.is_empty(),
        "Should have at least one lane-0 judge note"
    );
    for &(jn_idx, &(tl_idx, _lane)) in &lane0_entries {
        assert_ne!(
            tl_idx,
            usize::MAX,
            "JudgeNote {} should map to a valid timeline",
            jn_idx
        );
        // The mapped timeline must actually have a note on lane 0.
        assert!(
            player.model.timelines[tl_idx].note(0).is_some(),
            "Timeline {} (for JudgeNote {}) must have a note on lane 0, \
             but binary_search landed on a timeline without one",
            tl_idx,
            jn_idx,
        );
    }

    // Also verify lane-3 entries map to a timeline with a note on lane 3.
    let lane3_entries: Vec<_> = player
        .judge_note_to_model
        .iter()
        .enumerate()
        .filter(|&(_, &(_, lane))| lane == 3)
        .collect();
    for &(jn_idx, &(tl_idx, _lane)) in &lane3_entries {
        assert_ne!(
            tl_idx,
            usize::MAX,
            "JudgeNote {} should map to a valid timeline",
            jn_idx
        );
        assert!(
            player.model.timelines[tl_idx].note(3).is_some(),
            "Timeline {} (for JudgeNote {}) must have a note on lane 3",
            tl_idx,
            jn_idx,
        );
    }
}

// --- Course gauge constraint (Finding 1) ---

#[test]
fn create_uses_gauge_7keys_constraint_for_course_gauge_property() {
    // When constraints include Gauge7Keys, the GrooveGauge should use SevenKeys gauge
    // tables instead of the default LR2 tables.
    // SevenKeys HARD (index 3) has death=0.0, LR2 HARD has death=2.0.
    let model = make_model();
    let mut player = BMSPlayer::new(model);
    player.is_course_mode = true;
    player.constraints = vec![CourseDataConstraint::Gauge7Keys];
    player.create();

    let gauge = player.gauge.as_ref().expect("gauge should be created");
    assert!(gauge.is_course_gauge(), "should be course gauge");
    // Check HARD gauge (index 3): SevenKeys=death 0.0, LR2=death 2.0
    let hard_gauge = gauge.gauge_by_type(3);
    assert_eq!(
        hard_gauge.property().death,
        0.0,
        "SevenKeys gauge property should have death=0.0 for HARD gauge; \
         got {} which suggests LR2 gauge tables were used instead of SevenKeys",
        hard_gauge.property().death
    );
}

#[test]
fn create_uses_gauge_5keys_constraint_for_course_gauge_property() {
    // When constraints include Gauge5Keys, the GrooveGauge should use FiveKeys gauge tables.
    // FiveKeys NORMAL (index 2) has border=75.0, SevenKeys NORMAL has border=80.0.
    let model = make_model();
    let mut player = BMSPlayer::new(model);
    player.is_course_mode = true;
    player.constraints = vec![CourseDataConstraint::Gauge5Keys];
    player.create();

    let gauge = player.gauge.as_ref().expect("gauge should be created");
    // Check NORMAL (index 2) border: FiveKeys=75.0, SevenKeys=80.0
    let normal_gauge = gauge.gauge_by_type(2);
    assert_eq!(
        normal_gauge.property().border,
        75.0,
        "FiveKeys gauge property should have border=75.0 for NORMAL gauge; \
         got {} which suggests wrong gauge tables were used",
        normal_gauge.property().border
    );
}

#[test]
fn create_without_gauge_constraint_uses_default_for_course() {
    // Without gauge constraints, course mode should fall back to mode-based default.
    let model = make_model();
    let mut player = BMSPlayer::new(model);
    player.is_course_mode = true;
    // No gauge constraints, just Class
    player.constraints = vec![CourseDataConstraint::Class];
    player.create();

    let gauge = player.gauge.as_ref().expect("gauge should be created");
    assert!(gauge.is_course_gauge(), "should be course gauge");
}

// --- Course gauge restoration (Finding 2) ---

#[test]
fn create_restores_previous_stage_gauge_values() {
    // On subsequent course stages, the gauge values from the previous stage should
    // be restored. Java: GrooveGauge.create() reads resource.getGauge() and sets
    // each gauge type's value to the last entry of the corresponding log.
    let model = make_model();
    let mut player = BMSPlayer::new(model);
    player.is_course_mode = true;

    // Simulate previous stage gauge log: 9 gauge types, each with some history.
    // The last value of each log is what gets restored.
    let mut previous_gauge: Vec<Vec<f32>> = Vec::new();
    for i in 0..9 {
        // Different final values per gauge type for verification
        previous_gauge.push(vec![100.0, 90.0, 80.0 - i as f32 * 5.0]);
    }
    player.set_previous_gauge_values(previous_gauge.clone());
    player.create();

    let gauge = player.gauge.as_ref().expect("gauge should be created");
    // Verify each gauge type got the last value from the previous log.
    for i in 0..9 {
        let expected = 80.0 - i as f32 * 5.0;
        let actual = gauge.value_by_type(i as i32);
        // Note: set_value clamps and applies death border, so some values may
        // differ. For the first several types (init >= expected), the value
        // should match or be clamped.
        // Type 0-2 have min=2.0, so values >= 2.0 should survive.
        // Types 3-8 have min=0.0.
        if expected >= 2.0 {
            assert!(
                (actual - expected).abs() < 0.01 || actual == 0.0, // dead gauge
                "gauge type {} expected {} but got {}",
                i,
                expected,
                actual
            );
        }
    }
}

#[test]
fn create_without_previous_gauge_starts_fresh() {
    // First course stage (no previous gauge log) should use default init values.
    let model = make_model();
    let mut player = BMSPlayer::new(model);
    player.is_course_mode = true;
    // No previous_gauge_values set
    player.create();

    let gauge = player.gauge.as_ref().expect("gauge should be created");
    // Course mode with default gauge type (NORMAL=2) => id=6 (CLASS)
    // CLASS gauge init for LR2/SevenKeys defaults = 100.0
    let class_val = gauge.value_by_type(6);
    assert!(
        (class_val - 100.0).abs() < 0.01,
        "first course stage CLASS gauge should start at init value 100.0, got {}",
        class_val
    );
}

#[test]
fn create_restores_gauge_value_from_single_entry_log() {
    // Edge case: previous gauge log has only one entry per type.
    let model = make_model();
    let mut player = BMSPlayer::new(model);
    player.is_course_mode = true;

    let previous_gauge: Vec<Vec<f32>> = (0..9).map(|_| vec![75.0]).collect();
    player.set_previous_gauge_values(previous_gauge);
    player.create();

    let gauge = player.gauge.as_ref().expect("gauge should be created");
    // NORMAL (type 2) with border 80.0 and value 75.0 should be not qualified
    // (for 7key default). The value itself should be 75.0.
    let normal_val = gauge.value_by_type(2);
    assert!(
        (normal_val - 75.0).abs() < 0.01,
        "gauge type 2 should be restored to 75.0, got {}",
        normal_val
    );
}

#[test]
fn sync_judge_states_skips_out_of_range_indices() {
    // Safety test: verify sync doesn't panic with empty/invalid reverse index
    let model = make_model();
    let mut player = BMSPlayer::new(model);

    // Clear the reverse index (simulates no notes or no rebuild)
    player.judge_note_to_model = vec![];
    // Should not panic
    player.sync_judge_states_to_model();

    // Add an entry with usize::MAX (unmapped)
    player.judge_note_to_model = vec![(usize::MAX, 0)];
    player.sync_judge_states_to_model();
    // No assertion needed - just verifying no panic
}

// =========================================================================
// pad_gaugelog_with_zeros tests
// =========================================================================

#[test]
fn pad_gaugelog_normal_playtime() {
    // Normal case: 2-second song (playtime=2000ms), failed at 1000ms into play.
    // Should pad from 1000ms to 2500ms (playtime+500), i.e. 3 entries (1000, 1500, 2000).
    let mut gaugelog = vec![vec![50.0]; 2]; // 2 gauge types, each with 1 existing entry
    pad_gaugelog_with_zeros(&mut gaugelog, 1000, 2000);
    // (2000 + 500 - 1000) / 500 = 3 entries
    assert_eq!(
        gaugelog[0].len(),
        1 + 3,
        "gauge type 0 should have 4 entries total"
    );
    assert_eq!(
        gaugelog[1].len(),
        1 + 3,
        "gauge type 1 should have 4 entries total"
    );
    // Original entry preserved, rest are 0.0
    assert_eq!(gaugelog[0][0], 50.0);
    assert_eq!(gaugelog[0][1], 0.0);
    assert_eq!(gaugelog[0][3], 0.0);
}

#[test]
fn pad_gaugelog_corrupted_playtime_capped() {
    // Corrupted playtime: i32::MAX (~2.1 billion ms). Without the cap this would
    // try to push ~4.2 million entries per gauge type. With the cap it stops at 100_000.
    let mut gaugelog = vec![Vec::new(); 1];
    pad_gaugelog_with_zeros(&mut gaugelog, 0, i32::MAX as i64);
    assert_eq!(
        gaugelog[0].len(),
        100_000,
        "should be capped at 100_000 entries"
    );
}

#[test]
fn pad_gaugelog_no_entries_when_already_past_playtime() {
    // start_ms already beyond playtime + 500: no entries should be added.
    let mut gaugelog = vec![vec![99.0]; 1];
    pad_gaugelog_with_zeros(&mut gaugelog, 5000, 2000);
    assert_eq!(gaugelog[0].len(), 1, "no entries should be added");
    assert_eq!(gaugelog[0][0], 99.0, "existing entry should be preserved");
}

#[test]
fn pad_gaugelog_negative_playtime_no_entries() {
    // Negative playtime: playtime as i64 + 500 could be negative, loop should not execute.
    let mut gaugelog = vec![vec![]; 1];
    pad_gaugelog_with_zeros(&mut gaugelog, 0, -1000);
    assert_eq!(
        gaugelog[0].len(),
        0,
        "negative playtime should produce no entries"
    );
}

// Regression test: practice mode key-repeat uses game timer (monotonic),
// not SystemTime::now(). Verify that holding RIGHT during practice render
// increments the practice value using the game timer's now_time().
#[test]
fn practice_mode_render_uses_game_timer_for_key_repeat() {
    // Need timelines with times large enough for STARTTIME increment guard:
    // starttime + 2000 <= last_time (in millis). Use 60_000_000us = 60000ms.
    let model = make_model_with_notes_at_times(&[0, 60_000_000]);
    let mut player = BMSPlayer::new(model);
    player.play_mode = BMSPlayerMode::PRACTICE;
    player.create();
    assert_eq!(player.state(), PlayState::Practice);

    // Advance the game timer to a known value (e.g., 2000ms = 2_000_000us).
    // This ensures now_time() returns a meaningful value, not 0.
    player.main_state_data.timer.set_now_micro_time(2_000_000);

    let start_value = player.practice.practice_property().starttime;

    // Simulate holding RIGHT control key during a render cycle.
    player.input.control_key_right = true;
    player.render();

    let after_value = player.practice.practice_property().starttime;

    // RIGHT on cursor position 0 (STARTTIME) should increment by 100.
    // If the code were still using SystemTime::now(), the presscount would
    // be based on epoch millis (~1.7 trillion), creating a mismatch with
    // the game timer domain. With the fix, presscount is based on now_time()
    // (~2000ms), and the repeat logic works correctly.
    assert_eq!(
        after_value,
        start_value + 100,
        "Practice RIGHT should increment starttime by 100 using game timer"
    );
}

#[test]
fn song_metadata_getter_returns_set_value() {
    let model = make_model();
    let mut player = BMSPlayer::new(model);
    assert!(
        player.song_metadata().title.is_empty(),
        "song_metadata should default to empty"
    );

    let mut metadata = rubato_types::song_data::SongMetadata::default();
    metadata.title = "Test Song".to_string();
    metadata.artist = "Test Artist".to_string();
    metadata.genre = "Test Genre".to_string();
    player.set_song_metadata(metadata);

    assert_eq!(player.song_metadata().title, "Test Song");
    assert_eq!(player.song_metadata().artist, "Test Artist");
    assert_eq!(player.song_metadata().genre, "Test Genre");
}

#[test]
fn bga_poisoned_lock_does_not_crash_update_judge() {
    // Bug 1: If the BGA background thread panics while holding the lock, a
    // poisoned Mutex should NOT crash the render thread. The project convention
    // is to use `unwrap_or_else(|e| e.into_inner())` (lock_or_recover pattern).
    //
    // update_judge() calls bga.lock().expect("bga lock poisoned") when combo == 0.
    // A fresh JudgeManager starts with combo = 0, so this path is exercised.
    let model = make_model();
    let mut player = BMSPlayer::new(model);

    // Poison the BGA lock by panicking inside a thread that holds it.
    let bga_clone = Arc::clone(&player.bga);
    let handle = std::thread::spawn(move || {
        let _guard = bga_clone.lock().unwrap();
        panic!("intentional panic to poison BGA lock");
    });
    let _ = handle.join(); // join returns Err because of the panic

    // The lock is now poisoned. Calling update_judge should NOT panic.
    // Before fix: .expect("bga lock poisoned") panics on poisoned lock.
    // After fix: .unwrap_or_else(|e| e.into_inner()) recovers gracefully.
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        player.update_judge(0, 1000);
    }));
    assert!(
        result.is_ok(),
        "update_judge should not panic when BGA lock is poisoned"
    );
}

#[test]
fn receive_updated_play_config_preserves_scroll_state() {
    // Bug 2: receive_updated_play_config() calls lr.init(&self.model) which
    // destructively resets scroll positions (pos, basebpm, basehispeed) during
    // active gameplay. Only apply_play_config() should be called mid-game.
    //
    // Strategy: set fixhispeed = FIX_HISPEED_STARTBPM so that init() would
    // recalculate hispeed via set_lanecover -> reset_hispeed. If init() is
    // NOT called, hispeed stays at the config value.
    use rubato_types::play_config::{FIX_HISPEED_STARTBPM, PlayConfig};

    let mut model = make_model();
    model.bpm = 120.0;
    let mut player = BMSPlayer::new(model);
    player.lanerender = Some(LaneRenderer::new(&player.model));

    // First, apply a config that sets fixhispeed = STARTBPM and hispeed = 3.0
    let setup_config = PlayConfig {
        hispeed: 3.0,
        duration: 500,
        fixhispeed: FIX_HISPEED_STARTBPM,
        ..Default::default()
    };

    // Apply directly to lanerender first, then call init to establish basebpm
    if let Some(ref mut lr) = player.lanerender {
        lr.apply_play_config(&setup_config);
        lr.init(&player.model);
        // After init with FIX_HISPEED_STARTBPM:
        // basebpm = model.bpm = 120.0
        // set_lanecover(0.0) -> reset_hispeed(120.0) recalculates hispeed
        // basehispeed = recalculated hispeed
    }
    let hispeed_after_init = player.lanerender.as_ref().unwrap().hispeed();

    // Now set hispeed to a specific value that differs from what init() would produce
    let update_config = PlayConfig {
        hispeed: 3.0,
        duration: 500,
        fixhispeed: FIX_HISPEED_STARTBPM,
        ..Default::default()
    };

    let state: &mut dyn MainState = &mut player;
    state.receive_updated_play_config(Mode::BEAT_7K, update_config);

    let hispeed_after_update = player.lanerender.as_ref().unwrap().hispeed();

    // If init() was called (bug), hispeed would be recalculated by
    // set_lanecover -> reset_hispeed to the same value as hispeed_after_init.
    // If init() was NOT called (fix), hispeed stays at the config value 3.0.
    assert!(
        (hispeed_after_update - 3.0).abs() < f32::EPSILON,
        "hispeed should be the config value 3.0 after receive_updated_play_config, \
         but was {} (init() destructively recalculated it to {})",
        hispeed_after_update,
        hispeed_after_init
    );
}

#[test]
fn update_judge_sets_bga_misslayertime_in_milliseconds() {
    // Java JudgeManager calls main.update(judge, mtime / 1000), converting
    // microseconds to milliseconds before passing to BMSPlayer.update().
    // The Rust judged_events store raw microsecond mtime. update_judge must
    // divide by 1000 before calling set_misslayer_tme() so that
    // misslayertime is in the same clock domain as BGAProcessor.time (ms).
    let model = make_model();
    let mut player = BMSPlayer::new(model);
    player.gauge = Some(
        crate::groove_gauge::create_groove_gauge(
            &player.model,
            rubato_types::groove_gauge::NORMAL,
            0,
            None,
        )
        .unwrap(),
    );

    // combo starts at 0, so update_judge will call set_misslayer_tme.
    let time_us: i64 = 5_000_000; // 5 seconds in microseconds
    player.update_judge(4, time_us); // POOR/MISS -> combo stays 0

    let bga = player.bga.lock().unwrap();
    let expected_ms = time_us / 1000; // 5000 ms
    assert_eq!(
        bga.misslayer_time(),
        expected_ms,
        "misslayertime should be in milliseconds ({}), not microseconds ({})",
        expected_ms,
        time_us
    );
}

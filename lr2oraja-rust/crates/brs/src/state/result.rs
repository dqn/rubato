// Result state — ported from Java MusicResult.java.
//
// Displays play results (score, gauge graph, timing graph).
// Saves score to DB if applicable, then transitions to MusicSelect or CourseResult.

use tracing::info;

use bms_input::control_keys::ControlKeys;
use bms_skin::property_id::{
    TIMER_FADEOUT, TIMER_RESULT_UPDATESCORE, TIMER_RESULTGRAPH_BEGIN, TIMER_RESULTGRAPH_END,
    TIMER_STARTINPUT,
};

use crate::app_state::AppStateType;
use crate::state::{GameStateHandler, StateContext};

/// Default input delay in milliseconds (skin.getInput() placeholder).
const DEFAULT_INPUT_DELAY_MS: i64 = 500;
/// Default scene duration in milliseconds (skin.getScene() placeholder).
const DEFAULT_SCENE_DURATION_MS: i64 = 7000;
/// Default fadeout duration in milliseconds (skin.getFadeout() placeholder).
const DEFAULT_FADEOUT_DURATION_MS: i64 = 500;

/// Result state — displays play results and handles score persistence.
pub struct ResultState {
    /// Graph display type: 0 = gauge, 1 = timing.
    graph_type: i32,
    /// Whether this is a course (dan-i) play.
    is_course: bool,
    /// Current song index within the course (0-based).
    course_index: usize,
    /// Total songs in the course.
    course_total: usize,
    /// Whether the user cancelled (back to select).
    cancel: bool,
}

impl ResultState {
    pub fn new() -> Self {
        Self {
            graph_type: 0,
            is_course: false,
            course_index: 0,
            course_total: 0,
            cancel: false,
        }
    }
}

impl Default for ResultState {
    fn default() -> Self {
        Self::new()
    }
}

impl GameStateHandler for ResultState {
    fn create(&mut self, ctx: &mut StateContext) {
        self.cancel = false;
        self.graph_type = 0;
        info!("Result: create");

        // Save score to DB if update_score is set
        if ctx.resource.update_score
            && let Some(db) = ctx.database
        {
            let score = &ctx.resource.score_data;
            if let Err(e) = db.score_db.set_score_data(std::slice::from_ref(score)) {
                tracing::warn!("Result: failed to save score: {e}");
            }
            if let Err(e) = db
                .score_log_db
                .set_score_data_log(std::slice::from_ref(score))
            {
                tracing::warn!("Result: failed to save score log: {e}");
            }
            ctx.timer.set_timer_on(TIMER_RESULT_UPDATESCORE);

            // IR submission (fire-and-forget async)
            super::ir_submission::submit_score_to_ir(score, &score.sha256, score.mode);
        }

        // Load old score from DB
        if let Some(db) = ctx.database {
            let sha256 = &ctx.resource.score_data.sha256;
            let mode = ctx.resource.score_data.mode;
            match db.score_db.get_score_data(sha256, mode) {
                Ok(Some(old)) => ctx.resource.oldscore = old,
                Ok(None) => ctx.resource.oldscore = Default::default(),
                Err(e) => {
                    tracing::warn!("Result: failed to load old score: {e}");
                    ctx.resource.oldscore = Default::default();
                }
            }
        }

        // Course mode: accumulate scores, replays, and gauge logs
        if ctx.resource.is_course() {
            self.is_course = true;
            self.course_index = ctx.resource.course_index;
            self.course_total = ctx.resource.course_total();

            // Push current score to course accumulator
            if let Some(scores) = &mut ctx.resource.course_score_data {
                scores.push(ctx.resource.score_data.clone());
            }
            // Save replay data for course replay accumulation
            if let Some(replay) = ctx.resource.replay_data.take() {
                ctx.resource.course_replays.push(replay);
            }
            // Save gauge log for course gauge graph
            if let Some(last_gauge) = ctx.resource.gauge_log.last() {
                ctx.resource.course_gauges.push(last_gauge.clone());
            }

            info!(
                course_index = self.course_index,
                course_total = self.course_total,
                "Result: course mode active"
            );
        }

        // Start result graph animation timer
        ctx.timer.set_timer_on(TIMER_RESULTGRAPH_BEGIN);
    }

    fn render(&mut self, ctx: &mut StateContext) {
        let now = ctx.timer.now_time();

        // Enable input after initial delay
        if now > DEFAULT_INPUT_DELAY_MS {
            ctx.timer.switch_timer(TIMER_STARTINPUT, true);
        }

        // Check fadeout -> transition
        if ctx.timer.is_timer_on(TIMER_FADEOUT) {
            if ctx.timer.now_time_of(TIMER_FADEOUT) > DEFAULT_FADEOUT_DURATION_MS {
                let next = if self.cancel {
                    AppStateType::MusicSelect
                } else if self.is_course && self.course_index + 1 < self.course_total {
                    // More songs in course: go to next Decide
                    AppStateType::Decide
                } else if self.is_course {
                    // Last song in course: go to CourseResult
                    AppStateType::CourseResult
                } else {
                    AppStateType::MusicSelect
                };
                info!(next = %next, cancel = self.cancel, "Result: transition");
                *ctx.transition = Some(next);
            }
        } else if now > DEFAULT_SCENE_DURATION_MS {
            info!("Result: scene timer expired, starting fadeout");
            ctx.timer.set_timer_on(TIMER_FADEOUT);
            ctx.timer.set_timer_on(TIMER_RESULTGRAPH_END);
        }

        // Sync result state to shared game state for skin rendering
        if let Some(shared) = &mut ctx.shared_state {
            super::result_skin_state::sync_result_state(
                shared,
                &ctx.resource.score_data,
                &ctx.resource.oldscore,
                ctx.resource.maxcombo,
            );
        }
    }

    fn input(&mut self, ctx: &mut StateContext) {
        if ctx.timer.is_timer_on(TIMER_FADEOUT) || !ctx.timer.is_timer_on(TIMER_STARTINPUT) {
            return;
        }

        if let Some(input_state) = ctx.input_state {
            for key in &input_state.pressed_keys {
                match key {
                    ControlKeys::Enter => {
                        self.do_confirm(ctx);
                        return;
                    }
                    ControlKeys::Escape => {
                        self.do_cancel(ctx);
                        return;
                    }
                    ControlKeys::Up | ControlKeys::Down => {
                        self.toggle_graph_type();
                        return;
                    }
                    _ => {}
                }
            }
        }
    }

    fn shutdown(&mut self, ctx: &mut StateContext) {
        info!("Result: shutdown");
        // Advance course index when moving to next stage
        if self.is_course && !self.cancel && self.course_index + 1 < self.course_total {
            ctx.resource.course_index += 1;
            ctx.resource.load_course_stage();
        }
        // Clear course state when cancelling mid-course
        if self.cancel && self.is_course {
            ctx.resource.clear_course();
        }
    }
}

impl ResultState {
    fn do_confirm(&mut self, ctx: &mut StateContext) {
        if !ctx.timer.is_timer_on(TIMER_FADEOUT) && ctx.timer.is_timer_on(TIMER_STARTINPUT) {
            ctx.timer.set_timer_on(TIMER_FADEOUT);
            ctx.timer.set_timer_on(TIMER_RESULTGRAPH_END);
        }
    }

    fn do_cancel(&mut self, ctx: &mut StateContext) {
        if !ctx.timer.is_timer_on(TIMER_FADEOUT) && ctx.timer.is_timer_on(TIMER_STARTINPUT) {
            self.cancel = true;
            ctx.timer.set_timer_on(TIMER_FADEOUT);
            ctx.timer.set_timer_on(TIMER_RESULTGRAPH_END);
        }
    }

    fn toggle_graph_type(&mut self) {
        self.graph_type = if self.graph_type == 0 { 1 } else { 0 };
    }
}

/// Test helpers for inspecting internal state.
#[cfg(test)]
impl ResultState {
    pub(crate) fn graph_type(&self) -> i32 {
        self.graph_type
    }

    pub(crate) fn is_cancel(&self) -> bool {
        self.cancel
    }

    pub(crate) fn set_course(&mut self, index: usize, total: usize) {
        self.is_course = true;
        self.course_index = index;
        self.course_total = total;
    }

    pub(crate) fn confirm(&mut self, ctx: &mut StateContext) {
        self.do_confirm(ctx);
    }

    #[allow(dead_code)] // Used in tests
    pub(crate) fn cancel(&mut self, ctx: &mut StateContext) {
        self.do_cancel(ctx);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database_manager::DatabaseManager;
    use crate::input_mapper::InputState;
    use crate::player_resource::PlayerResource;
    use crate::timer_manager::TimerManager;
    use bms_config::{Config, PlayerConfig};
    use bms_rule::ClearType;
    use bms_rule::ScoreData;

    fn make_ctx<'a>(
        timer: &'a mut TimerManager,
        resource: &'a mut PlayerResource,
        config: &'a Config,
        player_config: &'a mut PlayerConfig,
        transition: &'a mut Option<AppStateType>,
    ) -> StateContext<'a> {
        StateContext {
            timer,
            resource,
            config,
            player_config,
            transition,
            keyboard_backend: None,
            database: None,
            input_state: None,
            skin_manager: None,
            sound_manager: None,
            received_chars: &[],
            bevy_images: None,
            shared_state: None,
            preview_music: None,
        }
    }

    fn make_score(sha256: &str, mode: i32, clear: ClearType, epg: i32) -> ScoreData {
        let mut sd = ScoreData::default();
        sd.sha256 = sha256.to_string();
        sd.mode = mode;
        sd.clear = clear;
        sd.epg = epg;
        sd
    }

    // --- create() tests ---

    #[test]
    fn create_saves_score_to_db_when_update_score() {
        let mut state = ResultState::new();
        let mut timer = TimerManager::new();
        let mut resource = PlayerResource::default();
        let config = Config::default();
        let mut player_config = PlayerConfig::default();
        let mut transition = None;

        resource.update_score = true;
        resource.score_data = make_score("abc123", 0, ClearType::Normal, 100);

        let db = DatabaseManager::open_in_memory().unwrap();

        let mut ctx = StateContext {
            timer: &mut timer,
            resource: &mut resource,
            config: &config,
            player_config: &mut player_config,
            transition: &mut transition,
            keyboard_backend: None,
            database: Some(&db),
            input_state: None,
            skin_manager: None,
            sound_manager: None,
            received_chars: &[],
            bevy_images: None,
            shared_state: None,
            preview_music: None,
        };
        state.create(&mut ctx);

        // Verify score was saved to DB
        let loaded = db.score_db.get_score_data("abc123", 0).unwrap();
        assert!(loaded.is_some());
        let loaded = loaded.unwrap();
        assert_eq!(loaded.epg, 100);
        assert_eq!(loaded.clear, ClearType::Normal);

        // Verify TIMER_RESULT_UPDATESCORE was activated
        assert!(timer.is_timer_on(TIMER_RESULT_UPDATESCORE));
    }

    #[test]
    fn create_does_not_save_when_update_score_false() {
        let mut state = ResultState::new();
        let mut timer = TimerManager::new();
        let mut resource = PlayerResource::default();
        let config = Config::default();
        let mut player_config = PlayerConfig::default();
        let mut transition = None;

        resource.update_score = false;
        resource.score_data = make_score("abc123", 0, ClearType::Normal, 100);

        let db = DatabaseManager::open_in_memory().unwrap();

        let mut ctx = StateContext {
            timer: &mut timer,
            resource: &mut resource,
            config: &config,
            player_config: &mut player_config,
            transition: &mut transition,
            keyboard_backend: None,
            database: Some(&db),
            input_state: None,
            skin_manager: None,
            sound_manager: None,
            received_chars: &[],
            bevy_images: None,
            shared_state: None,
            preview_music: None,
        };
        state.create(&mut ctx);

        // Score should NOT be in DB
        let loaded = db.score_db.get_score_data("abc123", 0).unwrap();
        assert!(loaded.is_none());
        assert!(!timer.is_timer_on(TIMER_RESULT_UPDATESCORE));
    }

    #[test]
    fn create_loads_old_score_from_db() {
        let mut state = ResultState::new();
        let mut timer = TimerManager::new();
        let mut resource = PlayerResource::default();
        let config = Config::default();
        let mut player_config = PlayerConfig::default();
        let mut transition = None;

        // Pre-populate DB with an old score
        let db = DatabaseManager::open_in_memory().unwrap();
        let old = make_score("sha_test", 0, ClearType::Hard, 200);
        db.score_db.set_score_data(&[old]).unwrap();

        // Current play score
        resource.update_score = false;
        resource.score_data = make_score("sha_test", 0, ClearType::Normal, 50);

        let mut ctx = StateContext {
            timer: &mut timer,
            resource: &mut resource,
            config: &config,
            player_config: &mut player_config,
            transition: &mut transition,
            keyboard_backend: None,
            database: Some(&db),
            input_state: None,
            skin_manager: None,
            sound_manager: None,
            received_chars: &[],
            bevy_images: None,
            shared_state: None,
            preview_music: None,
        };
        state.create(&mut ctx);

        // Old score should be loaded
        assert_eq!(resource.oldscore.epg, 200);
        assert_eq!(resource.oldscore.clear, ClearType::Hard);
    }

    #[test]
    fn create_sets_resultgraph_begin_timer() {
        let mut state = ResultState::new();
        let mut timer = TimerManager::new();
        let mut resource = PlayerResource::default();
        let config = Config::default();
        let mut player_config = PlayerConfig::default();
        let mut transition = None;

        let mut ctx = make_ctx(
            &mut timer,
            &mut resource,
            &config,
            &mut player_config,
            &mut transition,
        );
        state.create(&mut ctx);
        assert!(timer.is_timer_on(TIMER_RESULTGRAPH_BEGIN));
    }

    #[test]
    fn create_resets_cancel_and_graph_type() {
        let mut state = ResultState::new();
        state.cancel = true;
        state.graph_type = 1;

        let mut timer = TimerManager::new();
        let mut resource = PlayerResource::default();
        let config = Config::default();
        let mut player_config = PlayerConfig::default();
        let mut transition = None;

        let mut ctx = make_ctx(
            &mut timer,
            &mut resource,
            &config,
            &mut player_config,
            &mut transition,
        );
        state.create(&mut ctx);
        assert!(!state.is_cancel());
        assert_eq!(state.graph_type(), 0);
    }

    // --- render() tests ---

    #[test]
    fn render_enables_input_after_delay() {
        let mut state = ResultState::new();
        let mut timer = TimerManager::new();
        let mut resource = PlayerResource::default();
        let config = Config::default();
        let mut player_config = PlayerConfig::default();
        let mut transition = None;

        // Before delay
        timer.set_now_micro_time(400_000);
        let mut ctx = make_ctx(
            &mut timer,
            &mut resource,
            &config,
            &mut player_config,
            &mut transition,
        );
        state.render(&mut ctx);
        assert!(!timer.is_timer_on(TIMER_STARTINPUT));

        // After delay
        timer.set_now_micro_time(501_000);
        let mut ctx = make_ctx(
            &mut timer,
            &mut resource,
            &config,
            &mut player_config,
            &mut transition,
        );
        state.render(&mut ctx);
        assert!(timer.is_timer_on(TIMER_STARTINPUT));
    }

    #[test]
    fn render_starts_fadeout_after_scene_duration() {
        let mut state = ResultState::new();
        let mut timer = TimerManager::new();
        let mut resource = PlayerResource::default();
        let config = Config::default();
        let mut player_config = PlayerConfig::default();
        let mut transition = None;

        timer.set_now_micro_time(7_001_000);
        let mut ctx = make_ctx(
            &mut timer,
            &mut resource,
            &config,
            &mut player_config,
            &mut transition,
        );
        state.render(&mut ctx);
        assert!(timer.is_timer_on(TIMER_FADEOUT));
        assert!(timer.is_timer_on(TIMER_RESULTGRAPH_END));
    }

    #[test]
    fn render_transitions_to_select_after_fadeout() {
        let mut state = ResultState::new();
        let mut timer = TimerManager::new();
        let mut resource = PlayerResource::default();
        let config = Config::default();
        let mut player_config = PlayerConfig::default();
        let mut transition = None;

        // Set up FADEOUT timer
        timer.set_now_micro_time(1_000_000);
        timer.set_timer_on(TIMER_FADEOUT);

        // Advance past fadeout duration
        timer.set_now_micro_time(1_501_000);
        let mut ctx = make_ctx(
            &mut timer,
            &mut resource,
            &config,
            &mut player_config,
            &mut transition,
        );
        state.render(&mut ctx);
        assert_eq!(transition, Some(AppStateType::MusicSelect));
    }

    #[test]
    fn render_course_last_song_transitions_to_course_result() {
        let mut state = ResultState::new();
        state.set_course(3, 4); // last song (index 3 of 4)

        let mut timer = TimerManager::new();
        let mut resource = PlayerResource::default();
        let config = Config::default();
        let mut player_config = PlayerConfig::default();
        let mut transition = None;

        timer.set_now_micro_time(1_000_000);
        timer.set_timer_on(TIMER_FADEOUT);
        timer.set_now_micro_time(1_501_000);

        let mut ctx = make_ctx(
            &mut timer,
            &mut resource,
            &config,
            &mut player_config,
            &mut transition,
        );
        state.render(&mut ctx);
        assert_eq!(transition, Some(AppStateType::CourseResult));
    }

    #[test]
    fn render_course_mid_song_transitions_to_decide() {
        let mut state = ResultState::new();
        state.set_course(1, 4); // song 2 of 4

        let mut timer = TimerManager::new();
        let mut resource = PlayerResource::default();
        let config = Config::default();
        let mut player_config = PlayerConfig::default();
        let mut transition = None;

        timer.set_now_micro_time(1_000_000);
        timer.set_timer_on(TIMER_FADEOUT);
        timer.set_now_micro_time(1_501_000);

        let mut ctx = make_ctx(
            &mut timer,
            &mut resource,
            &config,
            &mut player_config,
            &mut transition,
        );
        state.render(&mut ctx);
        assert_eq!(transition, Some(AppStateType::Decide));
    }

    #[test]
    fn render_cancel_transitions_to_select_even_in_course() {
        let mut state = ResultState::new();
        state.set_course(1, 4);
        state.cancel = true;

        let mut timer = TimerManager::new();
        let mut resource = PlayerResource::default();
        let config = Config::default();
        let mut player_config = PlayerConfig::default();
        let mut transition = None;

        timer.set_now_micro_time(1_000_000);
        timer.set_timer_on(TIMER_FADEOUT);
        timer.set_now_micro_time(1_501_000);

        let mut ctx = make_ctx(
            &mut timer,
            &mut resource,
            &config,
            &mut player_config,
            &mut transition,
        );
        state.render(&mut ctx);
        assert_eq!(transition, Some(AppStateType::MusicSelect));
    }

    // --- input() tests ---

    #[test]
    fn input_enter_triggers_confirm() {
        let mut state = ResultState::new();
        let mut timer = TimerManager::new();
        let mut resource = PlayerResource::default();
        let config = Config::default();
        let mut player_config = PlayerConfig::default();
        let mut transition = None;

        timer.set_now_micro_time(600_000);
        timer.switch_timer(TIMER_STARTINPUT, true);

        let input_state = InputState {
            commands: vec![],
            pressed_keys: vec![ControlKeys::Enter],
        };

        let mut ctx = StateContext {
            timer: &mut timer,
            resource: &mut resource,
            config: &config,
            player_config: &mut player_config,
            transition: &mut transition,
            keyboard_backend: None,
            database: None,
            input_state: Some(&input_state),
            skin_manager: None,
            sound_manager: None,
            received_chars: &[],
            bevy_images: None,
            shared_state: None,
            preview_music: None,
        };
        state.input(&mut ctx);
        assert!(timer.is_timer_on(TIMER_FADEOUT));
        assert!(timer.is_timer_on(TIMER_RESULTGRAPH_END));
        assert!(!state.is_cancel());
    }

    #[test]
    fn input_escape_triggers_cancel() {
        let mut state = ResultState::new();
        let mut timer = TimerManager::new();
        let mut resource = PlayerResource::default();
        let config = Config::default();
        let mut player_config = PlayerConfig::default();
        let mut transition = None;

        timer.set_now_micro_time(600_000);
        timer.switch_timer(TIMER_STARTINPUT, true);

        let input_state = InputState {
            commands: vec![],
            pressed_keys: vec![ControlKeys::Escape],
        };

        let mut ctx = StateContext {
            timer: &mut timer,
            resource: &mut resource,
            config: &config,
            player_config: &mut player_config,
            transition: &mut transition,
            keyboard_backend: None,
            database: None,
            input_state: Some(&input_state),
            skin_manager: None,
            sound_manager: None,
            received_chars: &[],
            bevy_images: None,
            shared_state: None,
            preview_music: None,
        };
        state.input(&mut ctx);
        assert!(timer.is_timer_on(TIMER_FADEOUT));
        assert!(state.is_cancel());
    }

    #[test]
    fn input_up_toggles_graph_type() {
        let mut state = ResultState::new();
        let mut timer = TimerManager::new();
        let mut resource = PlayerResource::default();
        let config = Config::default();
        let mut player_config = PlayerConfig::default();
        let mut transition = None;

        timer.set_now_micro_time(600_000);
        timer.switch_timer(TIMER_STARTINPUT, true);

        assert_eq!(state.graph_type(), 0);

        let input_state = InputState {
            commands: vec![],
            pressed_keys: vec![ControlKeys::Up],
        };

        let mut ctx = StateContext {
            timer: &mut timer,
            resource: &mut resource,
            config: &config,
            player_config: &mut player_config,
            transition: &mut transition,
            keyboard_backend: None,
            database: None,
            input_state: Some(&input_state),
            skin_manager: None,
            sound_manager: None,
            received_chars: &[],
            bevy_images: None,
            shared_state: None,
            preview_music: None,
        };
        state.input(&mut ctx);
        assert_eq!(state.graph_type(), 1);

        // Toggle again with Down
        let input_state2 = InputState {
            commands: vec![],
            pressed_keys: vec![ControlKeys::Down],
        };
        let mut ctx2 = StateContext {
            timer: &mut timer,
            resource: &mut resource,
            config: &config,
            player_config: &mut player_config,
            transition: &mut transition,
            keyboard_backend: None,
            database: None,
            input_state: Some(&input_state2),
            skin_manager: None,
            sound_manager: None,
            received_chars: &[],
            bevy_images: None,
            shared_state: None,
            preview_music: None,
        };
        state.input(&mut ctx2);
        assert_eq!(state.graph_type(), 0);
    }

    #[test]
    fn input_ignored_before_input_enabled() {
        let mut state = ResultState::new();
        let mut timer = TimerManager::new();
        let mut resource = PlayerResource::default();
        let config = Config::default();
        let mut player_config = PlayerConfig::default();
        let mut transition = None;

        // TIMER_STARTINPUT not yet enabled
        let input_state = InputState {
            commands: vec![],
            pressed_keys: vec![ControlKeys::Enter],
        };

        let mut ctx = StateContext {
            timer: &mut timer,
            resource: &mut resource,
            config: &config,
            player_config: &mut player_config,
            transition: &mut transition,
            keyboard_backend: None,
            database: None,
            input_state: Some(&input_state),
            skin_manager: None,
            sound_manager: None,
            received_chars: &[],
            bevy_images: None,
            shared_state: None,
            preview_music: None,
        };
        state.input(&mut ctx);
        assert!(!timer.is_timer_on(TIMER_FADEOUT));
    }

    #[test]
    fn input_ignored_during_fadeout() {
        let mut state = ResultState::new();
        let mut timer = TimerManager::new();
        let mut resource = PlayerResource::default();
        let config = Config::default();
        let mut player_config = PlayerConfig::default();
        let mut transition = None;

        timer.set_now_micro_time(600_000);
        timer.switch_timer(TIMER_STARTINPUT, true);
        timer.set_timer_on(TIMER_FADEOUT);

        let input_state = InputState {
            commands: vec![],
            pressed_keys: vec![ControlKeys::Up],
        };

        let mut ctx = StateContext {
            timer: &mut timer,
            resource: &mut resource,
            config: &config,
            player_config: &mut player_config,
            transition: &mut transition,
            keyboard_backend: None,
            database: None,
            input_state: Some(&input_state),
            skin_manager: None,
            sound_manager: None,
            received_chars: &[],
            bevy_images: None,
            shared_state: None,
            preview_music: None,
        };
        state.input(&mut ctx);
        // graph_type should not change
        assert_eq!(state.graph_type(), 0);
    }

    #[test]
    fn confirm_sets_resultgraph_end_timer() {
        let mut state = ResultState::new();
        let mut timer = TimerManager::new();
        let mut resource = PlayerResource::default();
        let config = Config::default();
        let mut player_config = PlayerConfig::default();
        let mut transition = None;

        timer.set_now_micro_time(600_000);
        timer.switch_timer(TIMER_STARTINPUT, true);

        let mut ctx = make_ctx(
            &mut timer,
            &mut resource,
            &config,
            &mut player_config,
            &mut transition,
        );
        state.confirm(&mut ctx);
        assert!(timer.is_timer_on(TIMER_RESULTGRAPH_END));
    }

    #[test]
    fn create_without_db_does_not_panic() {
        let mut state = ResultState::new();
        let mut timer = TimerManager::new();
        let mut resource = PlayerResource::default();
        let config = Config::default();
        let mut player_config = PlayerConfig::default();
        let mut transition = None;

        resource.update_score = true;

        let mut ctx = make_ctx(
            &mut timer,
            &mut resource,
            &config,
            &mut player_config,
            &mut transition,
        );
        // Should not panic when database is None
        state.create(&mut ctx);
        assert!(!timer.is_timer_on(TIMER_RESULT_UPDATESCORE));
    }

    // --- Course accumulation tests ---

    fn setup_course_resource(resource: &mut PlayerResource, num_stages: usize) {
        use bms_model::BmsModel;
        let models: Vec<BmsModel> = (0..num_stages).map(|_| BmsModel::default()).collect();
        let dirs: Vec<std::path::PathBuf> = (0..num_stages)
            .map(|i| format!("/tmp/stage{i}").into())
            .collect();
        let course = bms_database::CourseData::default();
        resource.start_course(course, models, dirs);
    }

    #[test]
    fn create_course_accumulates_score() {
        let mut state = ResultState::new();
        let mut timer = TimerManager::new();
        let mut resource = PlayerResource::default();
        let config = Config::default();
        let mut player_config = PlayerConfig::default();
        let mut transition = None;

        setup_course_resource(&mut resource, 4);
        resource.score_data = make_score("sha1", 0, ClearType::Hard, 150);

        let mut ctx = make_ctx(
            &mut timer,
            &mut resource,
            &config,
            &mut player_config,
            &mut transition,
        );
        state.create(&mut ctx);

        // Verify score was pushed to course accumulator
        let scores = resource.course_score_data.as_ref().unwrap();
        assert_eq!(scores.len(), 1);
        assert_eq!(scores[0].epg, 150);
        assert_eq!(scores[0].clear, ClearType::Hard);

        // Verify course state was set on ResultState
        assert!(state.is_course);
        assert_eq!(state.course_index, 0);
        assert_eq!(state.course_total, 4);
    }

    #[test]
    fn create_course_accumulates_gauge_log() {
        let mut state = ResultState::new();
        let mut timer = TimerManager::new();
        let mut resource = PlayerResource::default();
        let config = Config::default();
        let mut player_config = PlayerConfig::default();
        let mut transition = None;

        setup_course_resource(&mut resource, 4);
        resource.gauge_log = vec![vec![0.5, 0.6], vec![0.7, 0.8]];

        let mut ctx = make_ctx(
            &mut timer,
            &mut resource,
            &config,
            &mut player_config,
            &mut transition,
        );
        state.create(&mut ctx);

        // Last gauge log entry should be pushed to course_gauges
        assert_eq!(resource.course_gauges.len(), 1);
        assert_eq!(resource.course_gauges[0], vec![0.7, 0.8]);
    }

    #[test]
    fn create_course_accumulates_replay() {
        use bms_replay::replay_data::ReplayData;

        let mut state = ResultState::new();
        let mut timer = TimerManager::new();
        let mut resource = PlayerResource::default();
        let config = Config::default();
        let mut player_config = PlayerConfig::default();
        let mut transition = None;

        setup_course_resource(&mut resource, 4);
        resource.replay_data = Some(ReplayData::default());

        let mut ctx = make_ctx(
            &mut timer,
            &mut resource,
            &config,
            &mut player_config,
            &mut transition,
        );
        state.create(&mut ctx);

        // Replay should be moved to course_replays
        assert_eq!(resource.course_replays.len(), 1);
        assert!(resource.replay_data.is_none());
    }

    #[test]
    fn shutdown_advances_course_index() {
        let mut state = ResultState::new();
        let mut timer = TimerManager::new();
        let mut resource = PlayerResource::default();
        let config = Config::default();
        let mut player_config = PlayerConfig::default();
        let mut transition = None;

        setup_course_resource(&mut resource, 4);
        // Simulate being on stage 1 (0-indexed)
        resource.course_index = 1;
        state.is_course = true;
        state.course_index = 1;
        state.course_total = 4;

        let mut ctx = make_ctx(
            &mut timer,
            &mut resource,
            &config,
            &mut player_config,
            &mut transition,
        );
        state.shutdown(&mut ctx);

        // course_index should be advanced
        assert_eq!(resource.course_index, 2);
    }

    #[test]
    fn shutdown_does_not_advance_on_last_song() {
        let mut state = ResultState::new();
        let mut timer = TimerManager::new();
        let mut resource = PlayerResource::default();
        let config = Config::default();
        let mut player_config = PlayerConfig::default();
        let mut transition = None;

        setup_course_resource(&mut resource, 4);
        state.is_course = true;
        state.course_index = 3;
        state.course_total = 4;

        let mut ctx = make_ctx(
            &mut timer,
            &mut resource,
            &config,
            &mut player_config,
            &mut transition,
        );
        state.shutdown(&mut ctx);

        // Should not advance past total
        assert_eq!(resource.course_index, 0); // unchanged from start_course default
    }

    #[test]
    fn shutdown_clears_course_on_cancel() {
        let mut state = ResultState::new();
        let mut timer = TimerManager::new();
        let mut resource = PlayerResource::default();
        let config = Config::default();
        let mut player_config = PlayerConfig::default();
        let mut transition = None;

        setup_course_resource(&mut resource, 4);
        state.is_course = true;
        state.cancel = true;
        state.course_index = 1;
        state.course_total = 4;

        let mut ctx = make_ctx(
            &mut timer,
            &mut resource,
            &config,
            &mut player_config,
            &mut transition,
        );
        state.shutdown(&mut ctx);

        // Course state should be cleared
        assert!(!resource.is_course());
        assert!(resource.course_data.is_none());
        assert!(resource.course_score_data.is_none());
    }

    #[test]
    fn create_non_course_does_not_set_course_fields() {
        let mut state = ResultState::new();
        let mut timer = TimerManager::new();
        let mut resource = PlayerResource::default();
        let config = Config::default();
        let mut player_config = PlayerConfig::default();
        let mut transition = None;

        // Not in course mode
        resource.score_data = make_score("sha1", 0, ClearType::Normal, 100);

        let mut ctx = make_ctx(
            &mut timer,
            &mut resource,
            &config,
            &mut player_config,
            &mut transition,
        );
        state.create(&mut ctx);

        assert!(!state.is_course);
        assert_eq!(state.course_index, 0);
        assert_eq!(state.course_total, 0);
    }
}

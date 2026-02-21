// CourseResult state — displays aggregated course results.
//
// Shows combined score across all course stages, determines overall clear type
// (worst of individual clears), evaluates trophies, and optionally saves to the database.

use tracing::info;

use bms_database::TrophyData;
use bms_input::control_keys::ControlKeys;
use bms_skin::property_id::{TIMER_FADEOUT, TIMER_STARTINPUT};

use crate::app_state::AppStateType;
use crate::skin_manager::SkinType;
use crate::state::{GameStateHandler, StateContext};

/// Course result state — aggregates and displays results for a course play session.
pub struct CourseResultState {
    fadeout_started: bool,
}

impl CourseResultState {
    pub fn new() -> Self {
        Self {
            fadeout_started: false,
        }
    }

    fn start_fadeout(&mut self, ctx: &mut StateContext) {
        if !ctx.timer.is_timer_on(TIMER_FADEOUT) && ctx.timer.is_timer_on(TIMER_STARTINPUT) {
            self.fadeout_started = true;
            ctx.timer.set_timer_on(TIMER_FADEOUT);
        }
    }
}

impl Default for CourseResultState {
    fn default() -> Self {
        Self::new()
    }
}

impl GameStateHandler for CourseResultState {
    fn create(&mut self, ctx: &mut StateContext) {
        self.fadeout_started = false;
        info!("CourseResult: create");

        if let Some(skin_mgr) = ctx.skin_manager.as_deref_mut() {
            skin_mgr.request_load(SkinType::CourseResult);
        }

        // Aggregate course scores if available
        if let Some(course_scores) = &ctx.resource.course_score_data
            && !course_scores.is_empty()
        {
            let mut aggregated = bms_rule::ScoreData::default();
            let mut worst_clear = bms_rule::ClearType::Max;

            for score in course_scores {
                aggregated.epg += score.epg;
                aggregated.lpg += score.lpg;
                aggregated.egr += score.egr;
                aggregated.lgr += score.lgr;
                aggregated.egd += score.egd;
                aggregated.lgd += score.lgd;
                aggregated.ebd += score.ebd;
                aggregated.lbd += score.lbd;
                aggregated.epr += score.epr;
                aggregated.lpr += score.lpr;
                aggregated.ems += score.ems;
                aggregated.lms += score.lms;
                aggregated.maxcombo += score.maxcombo;
                aggregated.notes += score.notes;

                if score.clear < worst_clear {
                    worst_clear = score.clear;
                }
            }

            aggregated.clear = worst_clear;
            // Use sha256 from the first stage for course score identification
            if let Some(first) = course_scores.first() {
                aggregated.sha256 = first.sha256.clone();
                aggregated.mode = first.mode;
            }
            aggregated.minbp = aggregated.ebd
                + aggregated.lbd
                + aggregated.epr
                + aggregated.lpr
                + aggregated.ems
                + aggregated.lms;

            info!(
                exscore = aggregated.exscore(),
                clear = ?aggregated.clear,
                stages = course_scores.len(),
                "CourseResult: aggregated scores"
            );

            // Evaluate trophy conditions from course data
            if let Some(course) = &ctx.resource.course_data
                && let Some(trophy) = evaluate_trophy(&aggregated, course)
            {
                info!(trophy_name = %trophy.name, "CourseResult: trophy earned");
                aggregated.trophy = trophy.name.clone();
            }

            ctx.resource.score_data = aggregated;
        }

        // Save course result to DB
        if ctx.resource.update_score
            && let Some(db) = ctx.database
        {
            let score = &ctx.resource.score_data;
            if let Err(e) = db.score_db.set_score_data(std::slice::from_ref(score)) {
                tracing::warn!("CourseResult: failed to save course score: {e}");
            }

            // IR course submission (fire-and-forget async)
            if let Some(course) = &ctx.resource.course_data {
                super::ir_submission::submit_course_score_to_ir(score, course);
            }
        }
    }

    fn render(&mut self, ctx: &mut StateContext) {
        let now = ctx.timer.now_time();
        let timing = ctx.skin_timing();

        // Enable input after initial delay (Java: getSkin().getInput())
        if now > timing.input_ms {
            ctx.timer.switch_timer(TIMER_STARTINPUT, true);
        }

        // Check fadeout -> transition (Java: getSkin().getFadeout())
        if ctx.timer.is_timer_on(TIMER_FADEOUT) {
            if ctx.timer.now_time_of(TIMER_FADEOUT) > timing.fadeout_ms {
                info!("CourseResult: transition to MusicSelect");
                *ctx.transition = Some(AppStateType::MusicSelect);
            }
        } else if now > timing.scene_ms {
            info!("CourseResult: scene timer expired, starting fadeout");
            self.fadeout_started = true;
            ctx.timer.set_timer_on(TIMER_FADEOUT);
        }

        // Sync course result state to shared game state for skin rendering
        if let Some(shared) = &mut ctx.shared_state {
            super::course_result_skin_state::sync_course_result_state(shared, ctx.resource);
        }
    }

    fn input(&mut self, ctx: &mut StateContext) {
        if ctx.timer.is_timer_on(TIMER_FADEOUT) || !ctx.timer.is_timer_on(TIMER_STARTINPUT) {
            return;
        }

        if let Some(input_state) = ctx.input_state {
            for key in &input_state.pressed_keys {
                match key {
                    ControlKeys::Enter | ControlKeys::Escape => {
                        self.start_fadeout(ctx);
                        return;
                    }
                    _ => {}
                }
            }
        }
    }

    fn shutdown(&mut self, ctx: &mut StateContext) {
        info!("CourseResult: shutdown");
        ctx.resource.clear_course();
    }
}

/// Evaluate trophy conditions for a course result.
///
/// Checks trophies in order from first to last (highest to lowest priority).
/// Returns the first trophy whose conditions are all satisfied.
///
/// Trophy evaluation uses `missrate` and `scorerate` from `TrophyData`:
/// - `missrate`: maximum allowed miss percentage (miss_count / total_notes * 100).
///   A trophy is satisfied if the player's miss rate is below this threshold.
/// - `scorerate`: minimum required score percentage (exscore / (notes * 2) * 100).
///   A trophy is satisfied if the player's score rate exceeds this threshold.
fn evaluate_trophy(
    score: &bms_rule::ScoreData,
    course: &bms_database::CourseData,
) -> Option<TrophyData> {
    for trophy in &course.trophy {
        let total_notes = score.notes;
        if total_notes == 0 {
            continue;
        }

        // Miss count: BD + PR + MS (early + late)
        let miss_count = score.ebd + score.lbd + score.epr + score.lpr + score.ems + score.lms;
        let miss_pct = (miss_count as f32 / total_notes as f32) * 100.0;
        let score_pct = (score.exscore() as f32 / (total_notes * 2) as f32) * 100.0;

        // Trophy is earned if miss rate is below missrate AND score rate is above scorerate
        if miss_pct < trophy.missrate && score_pct > trophy.scorerate {
            return Some(trophy.clone());
        }
    }
    None
}

#[cfg(test)]
impl CourseResultState {
    pub(crate) fn is_fadeout_started(&self) -> bool {
        self.fadeout_started
    }
}

#[cfg(test)]
mod tests {
    use super::*;
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
            download_handle: None,
        }
    }

    fn make_score(epg: i32, lpg: i32, egr: i32, lgr: i32, clear: ClearType) -> ScoreData {
        let mut sd = ScoreData::default();
        sd.epg = epg;
        sd.lpg = lpg;
        sd.egr = egr;
        sd.lgr = lgr;
        sd.clear = clear;
        sd
    }

    #[test]
    fn create_aggregates_course_scores() {
        let mut state = CourseResultState::new();
        let mut timer = TimerManager::new();
        let mut resource = PlayerResource::default();
        resource.course_score_data = Some(vec![
            make_score(100, 50, 30, 20, ClearType::Hard),
            make_score(80, 40, 20, 10, ClearType::Normal),
        ]);

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

        // Aggregated: epg=180, lpg=90, egr=50, lgr=30
        assert_eq!(resource.score_data.epg, 180);
        assert_eq!(resource.score_data.lpg, 90);
        assert_eq!(resource.score_data.egr, 50);
        assert_eq!(resource.score_data.lgr, 30);
        // Worst clear: Normal < Hard
        assert_eq!(resource.score_data.clear, ClearType::Normal);
    }

    #[test]
    fn render_enables_input_after_delay() {
        let mut state = CourseResultState::new();
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
    fn render_fadeout_transitions_to_music_select() {
        let mut state = CourseResultState::new();
        let mut timer = TimerManager::new();
        let mut resource = PlayerResource::default();
        let config = Config::default();
        let mut player_config = PlayerConfig::default();
        let mut transition = None;

        // Set up: FADEOUT timer on at time 1000ms
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
    fn input_confirm_starts_fadeout() {
        let mut state = CourseResultState::new();
        let mut timer = TimerManager::new();
        let mut resource = PlayerResource::default();
        let config = Config::default();
        let mut player_config = PlayerConfig::default();
        let mut transition = None;

        // Enable input
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
            download_handle: None,
        };
        state.input(&mut ctx);
        assert!(timer.is_timer_on(TIMER_FADEOUT));
        assert!(state.is_fadeout_started());
    }

    // --- Trophy evaluation tests ---

    #[test]
    fn evaluate_trophy_returns_first_matching() {
        let mut score = ScoreData::default();
        score.epg = 900;
        score.lpg = 100;
        score.egr = 50;
        score.lgr = 50;
        score.notes = 1000;
        // exscore = (900+100)*2 + 50+50 = 2100
        // score_pct = 2100 / 2000 * 100 = 105% (capped logically but test value)
        // miss_count = 0, miss_pct = 0%

        let course = bms_database::CourseData {
            name: "Test".to_string(),
            hash: vec![],
            constraint: vec![],
            trophy: vec![
                TrophyData {
                    name: "Gold".to_string(),
                    missrate: 1.0,   // miss must be < 1%
                    scorerate: 90.0, // score must be > 90%
                },
                TrophyData {
                    name: "Silver".to_string(),
                    missrate: 5.0,
                    scorerate: 80.0,
                },
            ],
            release: true,
        };

        let trophy = evaluate_trophy(&score, &course);
        assert!(trophy.is_some());
        assert_eq!(trophy.unwrap().name, "Gold");
    }

    #[test]
    fn evaluate_trophy_returns_second_when_first_fails() {
        let mut score = ScoreData::default();
        score.epg = 400;
        score.lpg = 100;
        score.egr = 200;
        score.lgr = 100;
        score.ebd = 10;
        score.lbd = 5;
        score.notes = 1000;
        // exscore = (400+100)*2 + 200+100 = 1300
        // score_pct = 1300/2000 * 100 = 65%
        // miss_count = 15, miss_pct = 1.5%

        let course = bms_database::CourseData {
            name: "Test".to_string(),
            hash: vec![],
            constraint: vec![],
            trophy: vec![
                TrophyData {
                    name: "Gold".to_string(),
                    missrate: 1.0, // miss must be < 1% — fails (1.5%)
                    scorerate: 90.0,
                },
                TrophyData {
                    name: "Silver".to_string(),
                    missrate: 5.0,   // miss must be < 5% — passes (1.5%)
                    scorerate: 60.0, // score must be > 60% — passes (65%)
                },
            ],
            release: true,
        };

        let trophy = evaluate_trophy(&score, &course);
        assert!(trophy.is_some());
        assert_eq!(trophy.unwrap().name, "Silver");
    }

    #[test]
    fn evaluate_trophy_returns_none_when_no_match() {
        let mut score = ScoreData::default();
        score.epg = 100;
        score.notes = 1000;
        score.ems = 100;
        // miss_pct = 10%

        let course = bms_database::CourseData {
            name: "Test".to_string(),
            hash: vec![],
            constraint: vec![],
            trophy: vec![TrophyData {
                name: "Gold".to_string(),
                missrate: 1.0,
                scorerate: 90.0,
            }],
            release: true,
        };

        let trophy = evaluate_trophy(&score, &course);
        assert!(trophy.is_none());
    }

    #[test]
    fn evaluate_trophy_returns_none_when_zero_notes() {
        let score = ScoreData::default();
        // notes = 0

        let course = bms_database::CourseData {
            name: "Test".to_string(),
            hash: vec![],
            constraint: vec![],
            trophy: vec![TrophyData {
                name: "Gold".to_string(),
                missrate: 100.0,
                scorerate: 0.0,
            }],
            release: true,
        };

        let trophy = evaluate_trophy(&score, &course);
        assert!(trophy.is_none());
    }

    #[test]
    fn evaluate_trophy_empty_trophies_returns_none() {
        let mut score = ScoreData::default();
        score.notes = 100;

        let course = bms_database::CourseData {
            name: "Test".to_string(),
            hash: vec![],
            constraint: vec![],
            trophy: vec![],
            release: true,
        };

        let trophy = evaluate_trophy(&score, &course);
        assert!(trophy.is_none());
    }

    // --- DB save tests ---

    #[test]
    fn create_saves_course_score_to_db() {
        use crate::database_manager::DatabaseManager;

        let mut state = CourseResultState::new();
        let mut timer = TimerManager::new();
        let mut resource = PlayerResource::default();
        let config = Config::default();
        let mut player_config = PlayerConfig::default();
        let mut transition = None;

        resource.update_score = true;
        resource.course_score_data = Some(vec![{
            let mut s = make_score(100, 50, 30, 20, ClearType::Hard);
            s.sha256 = "course_sha".to_string();
            s
        }]);

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
            download_handle: None,
        };

        state.create(&mut ctx);

        // Aggregated score should be saved to DB
        let loaded = db.score_db.get_score_data("course_sha", 0).unwrap();
        assert!(loaded.is_some());
        let loaded = loaded.unwrap();
        assert_eq!(loaded.epg, 100);
    }

    #[test]
    fn create_does_not_save_when_update_score_false() {
        use crate::database_manager::DatabaseManager;

        let mut state = CourseResultState::new();
        let mut timer = TimerManager::new();
        let mut resource = PlayerResource::default();
        let config = Config::default();
        let mut player_config = PlayerConfig::default();
        let mut transition = None;

        resource.update_score = false;
        resource.course_score_data = Some(vec![{
            let mut s = make_score(100, 50, 30, 20, ClearType::Hard);
            s.sha256 = "course_sha2".to_string();
            s
        }]);

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
            download_handle: None,
        };

        state.create(&mut ctx);

        // Score should NOT be in DB
        let loaded = db.score_db.get_score_data("course_sha2", 0).unwrap();
        assert!(loaded.is_none());
    }

    // --- shutdown tests ---

    #[test]
    fn shutdown_clears_course_state() {
        let mut state = CourseResultState::new();
        let mut timer = TimerManager::new();
        let mut resource = PlayerResource::default();
        let config = Config::default();
        let mut player_config = PlayerConfig::default();
        let mut transition = None;

        // Set up course state
        use bms_model::BmsModel;
        let models = vec![BmsModel::default(), BmsModel::default()];
        let dirs = vec!["/tmp/s0".into(), "/tmp/s1".into()];
        resource.start_course(
            bms_database::CourseData::default(),
            models,
            dirs,
            Vec::new(),
        );
        assert!(resource.is_course());

        let mut ctx = make_ctx(
            &mut timer,
            &mut resource,
            &config,
            &mut player_config,
            &mut transition,
        );
        state.shutdown(&mut ctx);

        // Course state should be fully cleared
        assert!(!resource.is_course());
        assert!(resource.course_data.is_none());
        assert!(resource.course_score_data.is_none());
        assert!(resource.course_replays.is_empty());
        assert!(resource.course_gauges.is_empty());
    }

    #[test]
    fn create_sets_trophy_name_on_aggregated_score() {
        let mut state = CourseResultState::new();
        let mut timer = TimerManager::new();
        let mut resource = PlayerResource::default();
        let config = Config::default();
        let mut player_config = PlayerConfig::default();
        let mut transition = None;

        let mut s1 = make_score(450, 50, 25, 25, ClearType::Hard);
        s1.notes = 500;
        let mut s2 = make_score(450, 50, 25, 25, ClearType::Hard);
        s2.notes = 500;
        resource.course_score_data = Some(vec![s1, s2]);

        // Aggregated: epg=900, lpg=100, egr=50, lgr=50, notes=1000
        // exscore = 2100, score_pct = 105%, miss_count = 0, miss_pct = 0%
        resource.course_data = Some(bms_database::CourseData {
            name: "Trophy Course".to_string(),
            hash: vec![],
            constraint: vec![],
            trophy: vec![TrophyData {
                name: "Diamond".to_string(),
                missrate: 1.0,
                scorerate: 90.0,
            }],
            release: true,
        });

        let mut ctx = make_ctx(
            &mut timer,
            &mut resource,
            &config,
            &mut player_config,
            &mut transition,
        );
        state.create(&mut ctx);

        assert_eq!(resource.score_data.trophy, "Diamond");
    }
}

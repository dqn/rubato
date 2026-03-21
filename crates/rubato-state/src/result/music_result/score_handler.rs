// Score database update logic for MusicResult.
// Extracted from mod.rs for navigability.
//
// Contains update_score_database() which handles:
// - Old score lookup and comparison
// - Timing distribution calculation
// - Course mode score accumulation
// - Score persistence to database

use log::info;

use rubato_core::clear_type::ClearType;
use rubato_core::score_data::ScoreData;

use super::super::{BMSPlayerModeType, FreqTrainerMenu};
use super::MusicResult;

impl MusicResult {
    pub(super) fn update_score_database(&mut self) {
        let newscore = self.resource.score_data().cloned();
        if newscore.is_none() {
            let total_notes = self.resource.bms_model().total_notes();
            if let Some(mut cscore) = self.resource.course_score_data().cloned() {
                cscore.minbp += total_notes;
                cscore.clear = ClearType::Failed.id();
                self.resource.set_course_score_data(cscore);
            }
            return;
        }
        let newscore = newscore.expect("newscore");

        let oldsc = self.main.play_data_accessor().read_score_data_model(
            self.resource.bms_model(),
            self.resource.player_config().play_settings.lnmode,
        );
        self.data.oldscore = oldsc.unwrap_or_default();

        let target_exscore = self
            .resource
            .target_score_data()
            .map(|s| s.exscore())
            .unwrap_or(0);
        self.data.score.set_target_score(
            self.data.oldscore.exscore(),
            target_exscore,
            self.resource.bms_model().total_notes(),
        );
        self.data.score.update_score(Some(&newscore));

        // duration average
        self.data.avgduration = newscore.timing_stats.avgjudge;
        self.data.avg = newscore.timing_stats.avg;
        self.data.stddev = newscore.timing_stats.stddev;
        self.data.timing_distribution.init();

        let model = self.resource.bms_model();
        let lanes = model.mode().map(|m| m.key()).unwrap_or(8);
        for tl in &model.timelines {
            for i in 0..lanes {
                let n = tl.note(i);
                if let Some(note) = n {
                    // Check if this is not an end LN in LN mode
                    let is_end_ln = (model.lnmode == 1
                        || (model.lnmode == 0
                            && model.lntype() == bms_model::bms_model::LNTYPE_LONGNOTE))
                        && note.is_long()
                        && note.is_end();
                    if !is_end_ln {
                        let state = note.state();
                        // play_time() returns milliseconds, matching Java's
                        // Note.getPlayTime(). TimingDistribution bins are in ms
                        // (range=150 covers -150ms..+150ms). This intentionally
                        // differs from ScoreData.timing_stats which uses
                        // micro_play_time() (microseconds) for finer-grained
                        // summary statistics.
                        let play_time = note.play_time();
                        if state >= 1 {
                            self.data.timing_distribution.add(play_time as i32);
                        }
                    }
                }
            }
        }
        self.data.timing_distribution.statistic_value_calculate();
        self.data.sync_timing_distribution_cache();

        // Course mode score accumulation
        if self.resource.course_bms_models().is_some() {
            self.accumulate_course_score(&newscore);
        }

        if FreqTrainerMenu::is_freq_trainer_enabled()
            && let Some(sd) = self.resource.score_data_mut()
        {
            sd.clear = ClearType::NoPlay.id();
        }

        if self.resource.play_mode().mode == BMSPlayerModeType::Play
            && !(FreqTrainerMenu::is_freq_trainer_enabled() && FreqTrainerMenu::is_freq_negative())
        {
            if let Some(sd) = self.resource.score_data() {
                self.main.play_data_accessor().write_score_data_model(
                    sd,
                    self.resource.bms_model(),
                    self.resource.player_config().play_settings.lnmode,
                    self.resource.is_update_score(),
                );
            }
        } else {
            info!(
                "Play mode is {:?}, score not registered",
                self.resource.play_mode().mode
            );
        }
    }

    fn accumulate_course_score(&mut self, newscore: &ScoreData) {
        if newscore.clear == ClearType::Failed.id()
            && let Some(sd) = self.resource.score_data_mut()
        {
            sd.clear = ClearType::NoPlay.id();
        }
        let mut cscore = self.resource.course_score_data().cloned();
        if cscore.is_none() {
            let mut new_cscore = ScoreData {
                minbp: 0,
                ..Default::default()
            };
            let mut notes = 0;
            if let Some(models) = self.resource.course_bms_models() {
                for mo in models {
                    notes += mo.total_notes();
                }
            }
            new_cscore.notes = notes;
            new_cscore.play_option.device_type = newscore.play_option.device_type;
            new_cscore.play_option.option = newscore.play_option.option;
            new_cscore.play_option.judge_algorithm = newscore.play_option.judge_algorithm;
            new_cscore.play_option.rule = newscore.play_option.rule;
            self.resource.set_course_score_data(new_cscore.clone());
            cscore = Some(new_cscore);
        }

        if let Some(ref mut cs) = cscore {
            cs.passnotes += newscore.passnotes;
            cs.judge_counts.epg += newscore.judge_counts.epg;
            cs.judge_counts.lpg += newscore.judge_counts.lpg;
            cs.judge_counts.egr += newscore.judge_counts.egr;
            cs.judge_counts.lgr += newscore.judge_counts.lgr;
            cs.judge_counts.egd += newscore.judge_counts.egd;
            cs.judge_counts.lgd += newscore.judge_counts.lgd;
            cs.judge_counts.ebd += newscore.judge_counts.ebd;
            cs.judge_counts.lbd += newscore.judge_counts.lbd;
            cs.judge_counts.epr += newscore.judge_counts.epr;
            cs.judge_counts.lpr += newscore.judge_counts.lpr;
            cs.judge_counts.ems += newscore.judge_counts.ems;
            cs.judge_counts.lms += newscore.judge_counts.lms;
            cs.minbp += newscore.minbp;
            cs.timing_stats.total_duration += newscore.timing_stats.total_duration;

            let last_gauge_val = self
                .resource
                .groove_gauge()
                .map(|g| g.value())
                .unwrap_or(0.0);
            if last_gauge_val > 0.0 {
                if self.resource.assist() > 0 {
                    if self.resource.assist() == 1 && cs.clear != ClearType::AssistEasy.id() {
                        cs.clear = ClearType::LightAssistEasy.id();
                    } else {
                        cs.clear = ClearType::AssistEasy.id();
                    }
                } else if !(cs.clear == ClearType::LightAssistEasy.id()
                    || cs.clear == ClearType::AssistEasy.id())
                    && let Some(models) = self.resource.course_bms_models()
                    && self.resource.course_index() == models.len() - 1
                {
                    let mut course_total_notes = 0;
                    for m in models {
                        course_total_notes += m.total_notes();
                    }
                    if course_total_notes == self.resource.maxcombo() {
                        if cs.judge_count(2, true) + cs.judge_count(2, false) == 0 {
                            if cs.judge_count(1, true) + cs.judge_count(1, false) == 0 {
                                cs.clear = ClearType::Max.id();
                            } else {
                                cs.clear = ClearType::Perfect.id();
                            }
                        } else {
                            cs.clear = ClearType::FullCombo.id();
                        }
                    } else {
                        cs.clear = self
                            .resource
                            .groove_gauge()
                            .map(|g| g.clear_type())
                            .unwrap_or(ClearType::Failed)
                            .id();
                    }
                }
            } else {
                cs.clear = ClearType::Failed.id();

                let current_idx = self.resource.course_index();
                let mut b = false;
                if let Some(models) = self.resource.course_bms_models() {
                    for (i, m) in models.iter().enumerate() {
                        if b {
                            cs.minbp += m.total_notes();
                        }
                        if i == current_idx {
                            b = true;
                        }
                    }
                }
            }

            self.resource.set_course_score_data(cs.clone());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::result::test_helpers::{TestMainControllerAccess, make_test_config};
    use crate::result::{BMSPlayerModeType, MainController, PlayerResource};
    use rubato_core::timer_manager::TimerManager;
    use rubato_types::player_resource_access::PlayerResourceAccess;
    use std::path::{Path, PathBuf};

    /// Configurable mock resource for accumulate_course_score tests.
    struct CourseScoreResourceAccess {
        config: rubato_types::config::Config,
        player_config: rubato_types::player_config::PlayerConfig,
        score_data: Option<ScoreData>,
        course_score_data: Option<ScoreData>,
        groove_gauge: Option<rubato_types::groove_gauge::GrooveGauge>,
        gauge: Option<Vec<Vec<f32>>>,
        assist: i32,
        course_index: usize,
        maxcombo: i32,
        replay_data: Option<rubato_core::replay_data::ReplayData>,
        course_replay: Vec<rubato_core::replay_data::ReplayData>,
        course_gauge: Vec<Vec<Vec<f32>>>,
    }

    impl CourseScoreResourceAccess {
        fn new(config: rubato_types::config::Config) -> Self {
            Self {
                config,
                player_config: rubato_types::player_config::PlayerConfig::default(),
                score_data: Some(ScoreData::default()),
                course_score_data: None,
                groove_gauge: None,
                gauge: None,
                assist: 0,
                course_index: 0,
                maxcombo: 0,
                replay_data: Some(rubato_core::replay_data::ReplayData::default()),
                course_replay: Vec::new(),
                course_gauge: Vec::new(),
            }
        }
    }

    impl rubato_types::player_resource_access::ConfigAccess for CourseScoreResourceAccess {
        fn config(&self) -> &rubato_types::config::Config {
            &self.config
        }
        fn player_config(&self) -> &rubato_types::player_config::PlayerConfig {
            &self.player_config
        }
        fn player_config_mut(&mut self) -> Option<&mut rubato_types::player_config::PlayerConfig> {
            Some(&mut self.player_config)
        }
    }

    impl rubato_types::player_resource_access::ScoreAccess for CourseScoreResourceAccess {
        fn score_data(&self) -> Option<&ScoreData> {
            self.score_data.as_ref()
        }
        fn rival_score_data(&self) -> Option<&ScoreData> {
            None
        }
        fn target_score_data(&self) -> Option<&ScoreData> {
            None
        }
        fn course_score_data(&self) -> Option<&ScoreData> {
            self.course_score_data.as_ref()
        }
        fn set_course_score_data(&mut self, score: ScoreData) {
            self.course_score_data = Some(score);
        }
        fn score_data_mut(&mut self) -> Option<&mut ScoreData> {
            self.score_data.as_mut()
        }
    }

    impl rubato_types::player_resource_access::SongAccess for CourseScoreResourceAccess {
        fn songdata(&self) -> Option<&rubato_types::song_data::SongData> {
            None
        }
        fn songdata_mut(&mut self) -> Option<&mut rubato_types::song_data::SongData> {
            None
        }
        fn set_songdata(&mut self, _data: Option<rubato_types::song_data::SongData>) {}
        fn course_song_data(&self) -> Vec<rubato_types::song_data::SongData> {
            vec![]
        }
    }

    impl rubato_types::player_resource_access::ReplayAccess for CourseScoreResourceAccess {
        fn replay_data(&self) -> Option<&rubato_core::replay_data::ReplayData> {
            self.replay_data.as_ref()
        }
        fn replay_data_mut(&mut self) -> Option<&mut rubato_core::replay_data::ReplayData> {
            self.replay_data.as_mut()
        }
        fn course_replay(&self) -> &[rubato_core::replay_data::ReplayData] {
            &self.course_replay
        }
        fn add_course_replay(&mut self, rd: rubato_core::replay_data::ReplayData) {
            self.course_replay.push(rd);
        }
        fn course_replay_mut(&mut self) -> &mut Vec<rubato_core::replay_data::ReplayData> {
            &mut self.course_replay
        }
    }

    impl rubato_types::player_resource_access::CourseAccess for CourseScoreResourceAccess {
        fn course_data(&self) -> Option<&rubato_types::course_data::CourseData> {
            None
        }
        fn course_index(&self) -> usize {
            self.course_index
        }
        fn next_course(&mut self) -> bool {
            false
        }
        fn constraint(&self) -> Vec<rubato_types::course_data::CourseDataConstraint> {
            vec![]
        }
        fn set_course_data(&mut self, _data: rubato_types::course_data::CourseData) {}
        fn clear_course_data(&mut self) {}
    }

    impl rubato_types::player_resource_access::GaugeAccess for CourseScoreResourceAccess {
        fn gauge(&self) -> Option<&Vec<Vec<f32>>> {
            self.gauge.as_ref()
        }
        fn groove_gauge(&self) -> Option<&rubato_types::groove_gauge::GrooveGauge> {
            self.groove_gauge.as_ref()
        }
        fn course_gauge(&self) -> &Vec<Vec<Vec<f32>>> {
            &self.course_gauge
        }
        fn add_course_gauge(&mut self, gauge: Vec<Vec<f32>>) {
            self.course_gauge.push(gauge);
        }
        fn course_gauge_mut(&mut self) -> &mut Vec<Vec<Vec<f32>>> {
            &mut self.course_gauge
        }
    }

    impl rubato_types::player_resource_access::PlayerStateAccess for CourseScoreResourceAccess {
        fn maxcombo(&self) -> i32 {
            self.maxcombo
        }
        fn org_gauge_option(&self) -> i32 {
            0
        }
        fn set_org_gauge_option(&mut self, _val: i32) {}
        fn assist(&self) -> i32 {
            self.assist
        }
        fn is_update_score(&self) -> bool {
            true
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
    }

    impl rubato_types::player_resource_access::SessionMutation for CourseScoreResourceAccess {
        fn clear(&mut self) {}
        fn set_bms_file(&mut self, _path: &Path, _mode_type: i32, _mode_id: i32) -> bool {
            false
        }
        fn set_course_bms_files(&mut self, _files: &[PathBuf]) -> bool {
            false
        }
        fn set_tablename(&mut self, _name: &str) {}
        fn set_tablelevel(&mut self, _level: &str) {}
        fn set_rival_score_data_option(&mut self, _score: Option<ScoreData>) {}
        fn set_chart_option_data(&mut self, _option: Option<rubato_core::replay_data::ReplayData>) {
        }
    }

    impl rubato_types::player_resource_access::MediaAccess for CourseScoreResourceAccess {
        fn reverse_lookup_data(&self) -> Vec<String> {
            vec![]
        }
        fn reverse_lookup_levels(&self) -> Vec<String> {
            vec![]
        }
    }

    impl PlayerResourceAccess for CourseScoreResourceAccess {
        fn into_any_send(self: Box<Self>) -> Box<dyn std::any::Any + Send> {
            self
        }
    }

    /// Create a BMSModel with `n` normal notes and total = 300.0.
    fn make_model_with_notes(n: usize) -> bms_model::bms_model::BMSModel {
        use bms_model::mode::Mode;
        use bms_model::note::Note;
        use bms_model::time_line::TimeLine;

        let mut model = bms_model::bms_model::BMSModel::new();
        model.set_mode(Mode::BEAT_7K);
        model.total = 300.0;
        let mut timelines = Vec::with_capacity(n);
        for i in 0..n {
            let mut tl = TimeLine::new(0.0, (i as i64) * 1_000_000, 8);
            tl.set_note(0, Some(Note::new_normal(1)));
            timelines.push(tl);
        }
        model.timelines = timelines;
        model
    }

    /// Build a MusicResult wired for accumulate_course_score testing.
    fn make_course_result(
        resource_access: CourseScoreResourceAccess,
        course_models: Vec<bms_model::bms_model::BMSModel>,
    ) -> MusicResult {
        let config = resource_access.config.clone();
        let main = MainController::new(Box::new(TestMainControllerAccess::new(config)));
        let mut resource = PlayerResource::new(
            Box::new(resource_access),
            rubato_core::bms_player_mode::BMSPlayerMode::new(BMSPlayerModeType::Play),
        );
        resource.course_bms_models = Some(course_models);
        MusicResult::new(main, resource, TimerManager::new())
    }

    /// Helper to create a GrooveGauge with a given gauge_type.
    fn make_groove_gauge(gauge_type: i32) -> rubato_types::groove_gauge::GrooveGauge {
        let model = make_model_with_notes(10);
        rubato_types::groove_gauge::GrooveGauge::new(
            &model,
            gauge_type,
            &rubato_types::gauge_property::GaugeProperty::SevenKeys,
        )
    }

    /// Helper to create a GrooveGauge with a given gauge_type and explicit value.
    /// Uses HARD gauge type (min=0.0) when value is 0.0 to allow zero gauge.
    fn make_groove_gauge_with_value(
        gauge_type: i32,
        value: f32,
    ) -> rubato_types::groove_gauge::GrooveGauge {
        let model = make_model_with_notes(10);
        let mut gg = rubato_types::groove_gauge::GrooveGauge::new(
            &model,
            gauge_type,
            &rubato_types::gauge_property::GaugeProperty::SevenKeys,
        );
        gg.set_value(value);
        gg
    }

    // --- 1. Failed clear handling ---

    #[test]
    fn test_accumulate_course_score_failed_clear_sets_noplay() {
        let config = make_test_config("cs-failed");
        let mut ra = CourseScoreResourceAccess::new(config);
        ra.score_data.as_mut().unwrap().clear = ClearType::Normal.id();

        let mut mr = make_course_result(ra, vec![make_model_with_notes(100)]);

        let mut newscore = ScoreData::default();
        newscore.clear = ClearType::Failed.id();

        mr.accumulate_course_score(&newscore);

        assert_eq!(
            mr.resource.score_data().unwrap().clear,
            ClearType::NoPlay.id(),
            "Failed newscore should set score_data.clear to NoPlay"
        );
    }

    #[test]
    fn test_accumulate_course_score_non_failed_preserves_clear() {
        let config = make_test_config("cs-nonfailed");
        let mut ra = CourseScoreResourceAccess::new(config);
        ra.score_data.as_mut().unwrap().clear = ClearType::Normal.id();

        let mut mr = make_course_result(ra, vec![make_model_with_notes(100)]);

        let mut newscore = ScoreData::default();
        newscore.clear = ClearType::Normal.id();

        mr.accumulate_course_score(&newscore);

        assert_eq!(
            mr.resource.score_data().unwrap().clear,
            ClearType::Normal.id(),
            "Non-failed newscore should not modify score_data.clear"
        );
    }

    // --- 2. First course song initialization ---

    #[test]
    fn test_accumulate_course_score_initializes_course_score_data() {
        let config = make_test_config("cs-init");
        let ra = CourseScoreResourceAccess::new(config);

        let models = vec![make_model_with_notes(100), make_model_with_notes(50)];
        let mut mr = make_course_result(ra, models);

        let mut newscore = ScoreData::default();
        newscore.clear = ClearType::Normal.id();
        newscore.minbp = 0;
        newscore.play_option.option = 42;
        newscore.play_option.device_type =
            Some(rubato_types::bms_player_input_device::Type::KEYBOARD);
        newscore.play_option.judge_algorithm =
            Some(rubato_types::judge_algorithm::JudgeAlgorithm::Combo);
        newscore.play_option.rule = Some(rubato_types::bms_player_rule::BMSPlayerRule::LR2);

        mr.accumulate_course_score(&newscore);

        let cs = mr
            .resource
            .course_score_data()
            .expect("course_score_data should be set");
        assert_eq!(
            cs.notes, 150,
            "course score notes should be sum of all model total_notes"
        );
        // minbp initialized to 0, not i32::MAX
        assert_eq!(
            cs.minbp,
            0 + 50,
            "course score minbp starts at 0, then adds newscore + remaining"
        );
        assert_eq!(cs.play_option.option, 42);
        assert_eq!(
            cs.play_option.device_type,
            Some(rubato_types::bms_player_input_device::Type::KEYBOARD)
        );
        assert_eq!(
            cs.play_option.judge_algorithm,
            Some(rubato_types::judge_algorithm::JudgeAlgorithm::Combo)
        );
        assert_eq!(
            cs.play_option.rule,
            Some(rubato_types::bms_player_rule::BMSPlayerRule::LR2)
        );
    }

    #[test]
    fn test_accumulate_course_score_reuses_existing_course_score() {
        let config = make_test_config("cs-reuse");
        let mut ra = CourseScoreResourceAccess::new(config);
        let mut existing = ScoreData::default();
        existing.notes = 200;
        existing.minbp = 5;
        existing.judge_counts.epg = 10;
        ra.course_score_data = Some(existing);
        // Provide gauge > 0 so the failed-gauge path is not taken
        ra.gauge = Some(vec![vec![50.0]]);
        ra.groove_gauge = Some(make_groove_gauge(0));
        // Not last song, so no clear type change in the gauge>0 path
        ra.course_index = 0;

        let models = vec![make_model_with_notes(100), make_model_with_notes(100)];
        let mut mr = make_course_result(ra, models);

        let mut newscore = ScoreData::default();
        newscore.clear = ClearType::Normal.id();
        newscore.minbp = 3;
        newscore.judge_counts.epg = 5;

        mr.accumulate_course_score(&newscore);

        let cs = mr.resource.course_score_data().unwrap();
        assert_eq!(
            cs.notes, 200,
            "existing course_score_data notes should be preserved"
        );
        assert_eq!(cs.minbp, 8, "minbp should be accumulated (5 + 3)");
        assert_eq!(
            cs.judge_counts.epg, 15,
            "epg should be accumulated (10 + 5)"
        );
    }

    // --- 3. Judge count accumulation ---

    #[test]
    fn test_accumulate_course_score_accumulates_all_judge_counts() {
        let config = make_test_config("cs-judges");
        let ra = CourseScoreResourceAccess::new(config);

        let mut mr = make_course_result(ra, vec![make_model_with_notes(100)]);

        let mut newscore = ScoreData::default();
        newscore.clear = ClearType::Normal.id();
        newscore.judge_counts.epg = 1;
        newscore.judge_counts.lpg = 2;
        newscore.judge_counts.egr = 3;
        newscore.judge_counts.lgr = 4;
        newscore.judge_counts.egd = 5;
        newscore.judge_counts.lgd = 6;
        newscore.judge_counts.ebd = 7;
        newscore.judge_counts.lbd = 8;
        newscore.judge_counts.epr = 9;
        newscore.judge_counts.lpr = 10;
        newscore.judge_counts.ems = 11;
        newscore.judge_counts.lms = 12;
        newscore.passnotes = 42;
        newscore.minbp = 3;
        newscore.timing_stats.total_duration = 5000;

        mr.accumulate_course_score(&newscore);

        let cs = mr.resource.course_score_data().unwrap();
        assert_eq!(cs.judge_counts.epg, 1);
        assert_eq!(cs.judge_counts.lpg, 2);
        assert_eq!(cs.judge_counts.egr, 3);
        assert_eq!(cs.judge_counts.lgr, 4);
        assert_eq!(cs.judge_counts.egd, 5);
        assert_eq!(cs.judge_counts.lgd, 6);
        assert_eq!(cs.judge_counts.ebd, 7);
        assert_eq!(cs.judge_counts.lbd, 8);
        assert_eq!(cs.judge_counts.epr, 9);
        assert_eq!(cs.judge_counts.lpr, 10);
        assert_eq!(cs.judge_counts.ems, 11);
        assert_eq!(cs.judge_counts.lms, 12);
        assert_eq!(cs.passnotes, 42);
        assert_eq!(cs.minbp, 3, "minbp should be accumulated (0 + 3)");
        assert_eq!(cs.timing_stats.total_duration, 5000);
    }

    #[test]
    fn test_accumulate_course_score_accumulates_across_two_calls() {
        let config = make_test_config("cs-two-calls");
        let mut ra = CourseScoreResourceAccess::new(config);
        // Provide gauge > 0 so the failed-gauge path is not taken
        ra.gauge = Some(vec![vec![50.0]]);
        ra.groove_gauge = Some(make_groove_gauge(0));
        // Not last song on either call, so clear type won't change
        ra.course_index = 0;

        let models = vec![
            make_model_with_notes(50),
            make_model_with_notes(50),
            make_model_with_notes(50),
        ];
        let mut mr = make_course_result(ra, models);

        let mut newscore1 = ScoreData::default();
        newscore1.clear = ClearType::Normal.id();
        newscore1.judge_counts.epg = 10;
        newscore1.judge_counts.lgr = 5;
        newscore1.minbp = 2;
        newscore1.passnotes = 20;
        newscore1.timing_stats.total_duration = 1000;
        mr.accumulate_course_score(&newscore1);

        let mut newscore2 = ScoreData::default();
        newscore2.clear = ClearType::Normal.id();
        newscore2.judge_counts.epg = 7;
        newscore2.judge_counts.lgr = 3;
        newscore2.minbp = 4;
        newscore2.passnotes = 15;
        newscore2.timing_stats.total_duration = 2000;
        mr.accumulate_course_score(&newscore2);

        let cs = mr.resource.course_score_data().unwrap();
        assert_eq!(
            cs.judge_counts.epg, 17,
            "epg should accumulate across calls"
        );
        assert_eq!(cs.judge_counts.lgr, 8, "lgr should accumulate across calls");
        assert_eq!(cs.minbp, 6, "minbp should accumulate across calls");
        assert_eq!(cs.passnotes, 35, "passnotes should accumulate across calls");
        assert_eq!(
            cs.timing_stats.total_duration, 3000,
            "total_duration should accumulate"
        );
    }

    // --- 4. Clear type with gauge > 0 and assist > 0 ---

    #[test]
    fn test_accumulate_course_score_assist_1_sets_light_assist_easy() {
        let config = make_test_config("cs-assist1");
        let mut ra = CourseScoreResourceAccess::new(config);
        ra.assist = 1;
        ra.gauge = Some(vec![vec![50.0]]);
        ra.groove_gauge = Some(make_groove_gauge(0));

        let mut mr = make_course_result(ra, vec![make_model_with_notes(100)]);

        let mut newscore = ScoreData::default();
        newscore.clear = ClearType::Normal.id();
        mr.accumulate_course_score(&newscore);

        let cs = mr.resource.course_score_data().unwrap();
        assert_eq!(
            cs.clear,
            ClearType::LightAssistEasy.id(),
            "assist=1 with gauge>0 should set LightAssistEasy"
        );
    }

    #[test]
    fn test_accumulate_course_score_assist_2_sets_assist_easy() {
        let config = make_test_config("cs-assist2");
        let mut ra = CourseScoreResourceAccess::new(config);
        ra.assist = 2;
        ra.gauge = Some(vec![vec![50.0]]);
        ra.groove_gauge = Some(make_groove_gauge(0));

        let mut mr = make_course_result(ra, vec![make_model_with_notes(100)]);

        let mut newscore = ScoreData::default();
        newscore.clear = ClearType::Normal.id();
        mr.accumulate_course_score(&newscore);

        let cs = mr.resource.course_score_data().unwrap();
        assert_eq!(
            cs.clear,
            ClearType::AssistEasy.id(),
            "assist=2 with gauge>0 should set AssistEasy"
        );
    }

    #[test]
    fn test_accumulate_course_score_assist_1_preserves_assist_easy() {
        let config = make_test_config("cs-assist1-nodown");
        let mut ra = CourseScoreResourceAccess::new(config);
        ra.assist = 1;
        ra.gauge = Some(vec![vec![50.0]]);
        ra.groove_gauge = Some(make_groove_gauge(0));
        let mut existing = ScoreData::default();
        existing.clear = ClearType::AssistEasy.id();
        existing.notes = 100;
        existing.minbp = 5;
        ra.course_score_data = Some(existing);

        let mut mr = make_course_result(ra, vec![make_model_with_notes(100)]);

        let mut newscore = ScoreData::default();
        newscore.clear = ClearType::Normal.id();
        newscore.minbp = 0;
        mr.accumulate_course_score(&newscore);

        let cs = mr.resource.course_score_data().unwrap();
        assert_eq!(
            cs.clear,
            ClearType::AssistEasy.id(),
            "assist=1 should not downgrade existing AssistEasy to LightAssistEasy"
        );
    }

    // --- 5. Clear type on last course song with no assist ---

    #[test]
    fn test_accumulate_course_score_last_song_max_clear() {
        let config = make_test_config("cs-max");
        let mut ra = CourseScoreResourceAccess::new(config);
        ra.assist = 0;
        ra.gauge = Some(vec![vec![50.0]]);
        ra.groove_gauge = Some(make_groove_gauge(0));
        ra.course_index = 1;
        ra.maxcombo = 200;

        let models = vec![make_model_with_notes(100), make_model_with_notes(100)];
        let mut mr = make_course_result(ra, models);

        let mut newscore = ScoreData::default();
        newscore.clear = ClearType::Normal.id();
        newscore.judge_counts.epg = 100;
        mr.accumulate_course_score(&newscore);

        let cs = mr.resource.course_score_data().unwrap();
        assert_eq!(
            cs.clear,
            ClearType::Max.id(),
            "All PG with full combo should be Max"
        );
    }

    #[test]
    fn test_accumulate_course_score_last_song_perfect_clear() {
        let config = make_test_config("cs-perfect");
        let mut ra = CourseScoreResourceAccess::new(config);
        ra.assist = 0;
        ra.gauge = Some(vec![vec![50.0]]);
        ra.groove_gauge = Some(make_groove_gauge(0));
        ra.course_index = 1;
        ra.maxcombo = 200;

        let models = vec![make_model_with_notes(100), make_model_with_notes(100)];
        let mut mr = make_course_result(ra, models);

        let mut newscore = ScoreData::default();
        newscore.clear = ClearType::Normal.id();
        newscore.judge_counts.epg = 90;
        newscore.judge_counts.egr = 10;
        mr.accumulate_course_score(&newscore);

        let cs = mr.resource.course_score_data().unwrap();
        assert_eq!(
            cs.clear,
            ClearType::Perfect.id(),
            "GR present but no GD with full combo should be Perfect"
        );
    }

    #[test]
    fn test_accumulate_course_score_last_song_fullcombo_clear() {
        let config = make_test_config("cs-fc");
        let mut ra = CourseScoreResourceAccess::new(config);
        ra.assist = 0;
        ra.gauge = Some(vec![vec![50.0]]);
        ra.groove_gauge = Some(make_groove_gauge(0));
        ra.course_index = 1;
        ra.maxcombo = 200;

        let models = vec![make_model_with_notes(100), make_model_with_notes(100)];
        let mut mr = make_course_result(ra, models);

        let mut newscore = ScoreData::default();
        newscore.clear = ClearType::Normal.id();
        newscore.judge_counts.epg = 80;
        newscore.judge_counts.egr = 10;
        newscore.judge_counts.egd = 10;
        mr.accumulate_course_score(&newscore);

        let cs = mr.resource.course_score_data().unwrap();
        assert_eq!(
            cs.clear,
            ClearType::FullCombo.id(),
            "GD present with full combo should be FullCombo"
        );
    }

    #[test]
    fn test_accumulate_course_score_last_song_gauge_based_clear() {
        let config = make_test_config("cs-gauge-clear");
        let mut ra = CourseScoreResourceAccess::new(config);
        ra.assist = 0;
        ra.gauge = Some(vec![vec![50.0]]);
        ra.groove_gauge = Some(make_groove_gauge(0));
        ra.course_index = 1;
        ra.maxcombo = 150; // less than 200 total notes

        let models = vec![make_model_with_notes(100), make_model_with_notes(100)];
        let mut mr = make_course_result(ra, models);

        let mut newscore = ScoreData::default();
        newscore.clear = ClearType::Normal.id();
        newscore.judge_counts.epg = 80;
        mr.accumulate_course_score(&newscore);

        let cs = mr.resource.course_score_data().unwrap();
        let expected = mr.resource.groove_gauge().unwrap().clear_type().id();
        assert_eq!(
            cs.clear, expected,
            "Non-fullcombo last song should use groove_gauge clear_type"
        );
    }

    #[test]
    fn test_accumulate_course_score_not_last_song_no_clear_change() {
        let config = make_test_config("cs-notlast");
        let mut ra = CourseScoreResourceAccess::new(config);
        ra.assist = 0;
        ra.gauge = Some(vec![vec![50.0]]);
        ra.groove_gauge = Some(make_groove_gauge(0));
        ra.course_index = 0; // not last in a 3-song course
        ra.maxcombo = 300;

        let models = vec![
            make_model_with_notes(100),
            make_model_with_notes(100),
            make_model_with_notes(100),
        ];
        let mut mr = make_course_result(ra, models);

        let mut newscore = ScoreData::default();
        newscore.clear = ClearType::Normal.id();
        newscore.judge_counts.epg = 100;
        mr.accumulate_course_score(&newscore);

        let cs = mr.resource.course_score_data().unwrap();
        assert_eq!(
            cs.clear,
            ClearType::NoPlay.id(),
            "Not last song should not change clear type"
        );
    }

    // --- 6. Failed gauge (gauge <= 0) ---

    #[test]
    fn test_accumulate_course_score_gauge_zero_sets_failed() {
        let config = make_test_config("cs-gauge0");
        let mut ra = CourseScoreResourceAccess::new(config);
        ra.assist = 0;
        ra.gauge = Some(vec![vec![0.0]]);
        ra.groove_gauge = Some(make_groove_gauge_with_value(3, 0.0));
        ra.course_index = 0;

        let models = vec![
            make_model_with_notes(50),
            make_model_with_notes(60),
            make_model_with_notes(70),
        ];
        let mut mr = make_course_result(ra, models);

        let mut newscore = ScoreData::default();
        newscore.clear = ClearType::Normal.id();
        newscore.minbp = 10;
        mr.accumulate_course_score(&newscore);

        let cs = mr.resource.course_score_data().unwrap();
        assert_eq!(
            cs.clear,
            ClearType::Failed.id(),
            "gauge<=0 should set Failed"
        );
        // minbp = 0 (init) + 10 (newscore) + 60 (song 1) + 70 (song 2)
        assert_eq!(
            cs.minbp,
            10 + 60 + 70,
            "Failed gauge should add remaining songs' total_notes to minbp"
        );
    }

    #[test]
    fn test_accumulate_course_score_gauge_zero_middle_song() {
        let config = make_test_config("cs-gauge0-mid");
        let mut ra = CourseScoreResourceAccess::new(config);
        ra.assist = 0;
        ra.gauge = Some(vec![vec![0.0]]);
        ra.groove_gauge = Some(make_groove_gauge_with_value(3, 0.0));
        ra.course_index = 1;

        let models = vec![
            make_model_with_notes(40),
            make_model_with_notes(50),
            make_model_with_notes(60),
            make_model_with_notes(70),
        ];
        let mut mr = make_course_result(ra, models);

        let mut newscore = ScoreData::default();
        newscore.clear = ClearType::Normal.id();
        newscore.minbp = 5;
        mr.accumulate_course_score(&newscore);

        let cs = mr.resource.course_score_data().unwrap();
        assert_eq!(cs.clear, ClearType::Failed.id());
        // remaining songs after index 1: songs at index 2 (60) and 3 (70)
        assert_eq!(
            cs.minbp,
            5 + 60 + 70,
            "Failed gauge mid-course should add notes from songs after current index"
        );
    }

    #[test]
    fn test_accumulate_course_score_gauge_zero_last_song() {
        let config = make_test_config("cs-gauge0-last");
        let mut ra = CourseScoreResourceAccess::new(config);
        ra.assist = 0;
        ra.gauge = Some(vec![vec![0.0]]);
        ra.groove_gauge = Some(make_groove_gauge_with_value(3, 0.0));
        ra.course_index = 2;

        let models = vec![
            make_model_with_notes(40),
            make_model_with_notes(50),
            make_model_with_notes(60),
        ];
        let mut mr = make_course_result(ra, models);

        let mut newscore = ScoreData::default();
        newscore.clear = ClearType::Normal.id();
        newscore.minbp = 5;
        mr.accumulate_course_score(&newscore);

        let cs = mr.resource.course_score_data().unwrap();
        assert_eq!(cs.clear, ClearType::Failed.id());
        assert_eq!(
            cs.minbp, 5,
            "Failed gauge on last song should not add extra notes to minbp"
        );
    }

    // --- 7. No gauge data defaults ---

    #[test]
    fn test_accumulate_course_score_no_gauge_data_defaults_to_failed() {
        let config = make_test_config("cs-no-gauge");
        let mut ra = CourseScoreResourceAccess::new(config);
        ra.assist = 0;
        ra.gauge = None;
        ra.groove_gauge = None;
        ra.course_index = 0;

        let models = vec![make_model_with_notes(100), make_model_with_notes(50)];
        let mut mr = make_course_result(ra, models);

        let mut newscore = ScoreData::default();
        newscore.clear = ClearType::Normal.id();
        newscore.minbp = 2;
        mr.accumulate_course_score(&newscore);

        let cs = mr.resource.course_score_data().unwrap();
        assert_eq!(
            cs.clear,
            ClearType::Failed.id(),
            "No gauge data should take the failed path (gauge defaults to 0.0)"
        );
        assert_eq!(cs.minbp, 2 + 50);
    }

    // --- 8. Assist-easy clear is sticky ---

    #[test]
    fn test_accumulate_course_score_assist_easy_sticky() {
        let config = make_test_config("cs-sticky-ae");
        let mut ra = CourseScoreResourceAccess::new(config);
        ra.assist = 0;
        ra.gauge = Some(vec![vec![50.0]]);
        ra.groove_gauge = Some(make_groove_gauge(0));
        ra.course_index = 1;
        ra.maxcombo = 200;
        let mut existing = ScoreData::default();
        existing.clear = ClearType::AssistEasy.id();
        existing.notes = 200;
        existing.minbp = 3;
        ra.course_score_data = Some(existing);

        let models = vec![make_model_with_notes(100), make_model_with_notes(100)];
        let mut mr = make_course_result(ra, models);

        let mut newscore = ScoreData::default();
        newscore.clear = ClearType::Normal.id();
        newscore.judge_counts.epg = 100;
        newscore.minbp = 0;
        mr.accumulate_course_score(&newscore);

        let cs = mr.resource.course_score_data().unwrap();
        assert_eq!(
            cs.clear,
            ClearType::AssistEasy.id(),
            "AssistEasy should be sticky -- not overwritten by gauge-based clear"
        );
    }

    #[test]
    fn test_accumulate_course_score_light_assist_easy_sticky() {
        let config = make_test_config("cs-sticky-lae");
        let mut ra = CourseScoreResourceAccess::new(config);
        ra.assist = 0;
        ra.gauge = Some(vec![vec![50.0]]);
        ra.groove_gauge = Some(make_groove_gauge(0));
        ra.course_index = 1;
        ra.maxcombo = 200;
        let mut existing = ScoreData::default();
        existing.clear = ClearType::LightAssistEasy.id();
        existing.notes = 200;
        existing.minbp = 3;
        ra.course_score_data = Some(existing);

        let models = vec![make_model_with_notes(100), make_model_with_notes(100)];
        let mut mr = make_course_result(ra, models);

        let mut newscore = ScoreData::default();
        newscore.clear = ClearType::Normal.id();
        newscore.judge_counts.epg = 100;
        newscore.minbp = 0;
        mr.accumulate_course_score(&newscore);

        let cs = mr.resource.course_score_data().unwrap();
        assert_eq!(
            cs.clear,
            ClearType::LightAssistEasy.id(),
            "LightAssistEasy should be sticky"
        );
    }
}

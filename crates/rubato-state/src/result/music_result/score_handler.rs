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
                        let play_time = note.play_time();
                        if state >= 1 {
                            self.data.timing_distribution.add(play_time);
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

            let gauge_type = self
                .resource
                .groove_gauge()
                .map(|g| g.gauge_type() as usize)
                .unwrap_or(0);
            let last_gauge_val = self
                .resource
                .gauge()
                .and_then(|gd| gd.get(gauge_type))
                .and_then(|g| g.last().copied())
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

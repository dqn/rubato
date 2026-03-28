use std::path::Path;

use bms::model::bms_model::BMSModel;

use rubato_types::course_data::CourseDataConstraint;
use rubato_types::replay_data::ReplayData;
use rubato_types::score_data::ScoreData;

use super::{PlayDataAccessor, REPLAY};

impl PlayDataAccessor {
    pub fn read_score_data_model(&self, model: &BMSModel, lnmode: i32) -> Option<ScoreData> {
        let hash = &model.sha256;
        let ln = model.contains_undefined_long_note();
        self.read_score_data_by_hash(hash, ln, lnmode)
    }

    /// Write score data for a single BMSModel (delegates to write_score_data).
    pub fn write_score_data_model(
        &self,
        newscore: &ScoreData,
        model: &BMSModel,
        lnmode: i32,
        update_score: bool,
    ) {
        let hash = &model.sha256;
        let contains_undefined_ln = model.contains_undefined_long_note();
        let total_notes = model.total_notes();
        // Calculate last note time in microseconds
        let last_note_time_us = {
            let keys = model.mode().map(|m| m.key()).unwrap_or(0);
            let mut time: i64 = 0;
            for tl in &model.timelines {
                for i in 0..keys {
                    if tl.note(i).is_some_and(|n| n.state() != 0) {
                        time = tl.micro_time();
                    }
                }
            }
            time
        };
        let ctx = super::core::ScoreWriteContext {
            hash,
            contains_undefined_ln,
            total_notes,
            lnmode,
            update_score,
            last_note_time_us,
        };
        self.write_score_data(newscore, &ctx);
    }

    /// Check if replay data exists for a single BMSModel.
    pub fn exists_replay_data_model(&self, model: &BMSModel, lnmode: i32, index: i32) -> bool {
        let ln = model.contains_undefined_long_note();
        self.exists_replay_data(&model.sha256, ln, lnmode, index)
    }

    /// Write replay data for a single BMSModel.
    pub fn write_replay_data_model(
        &self,
        rd: &mut ReplayData,
        model: &BMSModel,
        lnmode: i32,
        index: i32,
    ) -> anyhow::Result<()> {
        let ln = model.contains_undefined_long_note();
        self.write_replay_data(rd, &model.sha256, ln, lnmode, index)
    }

    /// Delete score data for a single BMSModel.
    pub fn delete_score_data_model(&self, model: &BMSModel, lnmode: i32) {
        self.delete_score_data(&model.sha256, model.contains_undefined_long_note(), lnmode);
    }

    // ========================================================================
    // Course methods (multiple BMSModels)
    // ========================================================================

    /// Read score data for a course (multiple models).
    pub fn read_score_data_course(
        &self,
        models: &[BMSModel],
        lnmode: i32,
        option: i32,
        constraint: &[CourseDataConstraint],
    ) -> Option<ScoreData> {
        let hash = Self::course_hash(models);
        let ln = models.iter().any(|m| m.contains_undefined_long_note());
        let (hispeed, judge, gauge) = Self::compute_constraint_values(constraint);
        let mode_val = (if ln { lnmode } else { 0 })
            + option * 10
            + hispeed * 100
            + judge * 1000
            + gauge * 10000;
        self.scoredb.as_ref()?.score_data(&hash, mode_val)
    }

    /// Write score data for a course (delegates to write_score_data_for_course).
    pub fn write_score_data_course(
        &self,
        newscore: &ScoreData,
        models: &[BMSModel],
        lnmode: i32,
        option: i32,
        constraint: &[CourseDataConstraint],
        update_score: bool,
    ) {
        let hashes: Vec<&str> = models.iter().map(|m| m.sha256.as_str()).collect();
        let total_notes: i32 = models.iter().map(|m| m.total_notes()).sum();
        let ln = models.iter().any(|m| m.contains_undefined_long_note());
        let ctx = super::core::CourseScoreWriteContext {
            hashes: &hashes,
            total_notes,
            ln,
            lnmode,
            option,
            constraint,
            update_score,
        };
        self.write_score_data_for_course(newscore, &ctx);
    }

    /// Check if replay data exists for a course.
    pub fn exists_replay_data_course(
        &self,
        models: &[BMSModel],
        lnmode: i32,
        index: i32,
        constraint: &[CourseDataConstraint],
    ) -> bool {
        let hashes: Vec<String> = models.iter().map(|m| m.sha256.clone()).collect();
        let hash_refs: Vec<&str> = hashes.iter().map(|s| s.as_str()).collect();
        let ln = models.iter().any(|m| m.contains_undefined_long_note());
        let path = format!(
            "{}.brd",
            self.get_replay_data_file_path_course(&hash_refs, ln, lnmode, index, constraint)
        );
        Path::new(&path).exists()
    }

    /// Read course replay data (array of ReplayData).
    pub fn read_replay_data_course(
        &self,
        models: &[BMSModel],
        lnmode: i32,
        index: i32,
        constraint: &[CourseDataConstraint],
    ) -> Option<Vec<ReplayData>> {
        if !self.exists_replay_data_course(models, lnmode, index, constraint) {
            return None;
        }
        let hashes: Vec<String> = models.iter().map(|m| m.sha256.clone()).collect();
        let hash_refs: Vec<&str> = hashes.iter().map(|s| s.as_str()).collect();
        let ln = models.iter().any(|m| m.contains_undefined_long_note());
        let path = format!(
            "{}.brd",
            self.get_replay_data_file_path_course(&hash_refs, ln, lnmode, index, constraint)
        );
        match ReplayData::read_brd_course(Path::new(&path)) {
            Ok(rds) => Some(rds),
            Err(e) => {
                log::error!("Failed to read course replay data: {}", e);
                None
            }
        }
    }

    /// Write course replay data (array of ReplayData).
    pub fn write_replay_data_course(
        &self,
        rds: &mut [ReplayData],
        models: &[BMSModel],
        lnmode: i32,
        index: i32,
        constraint: &[CourseDataConstraint],
    ) -> anyhow::Result<()> {
        let hashes: Vec<String> = models.iter().map(|m| m.sha256.clone()).collect();
        let hash_refs: Vec<&str> = hashes.iter().map(|s| s.as_str()).collect();
        let ln = models.iter().any(|m| m.contains_undefined_long_note());
        let path = format!(
            "{}.brd",
            self.get_replay_data_file_path_course(&hash_refs, ln, lnmode, index, constraint)
        );
        ReplayData::write_brd_course(rds, Path::new(&path))?;
        log::info!("Course replay saved: {}", path);
        Ok(())
    }

    // ========================================================================
    // Course file path helpers
    // ========================================================================

    pub(super) fn get_replay_data_file_path_course(
        &self,
        hashes: &[&str],
        ln: bool,
        lnmode: i32,
        index: i32,
        constraint: &[CourseDataConstraint],
    ) -> String {
        // Course hash: first 10 chars of each model's hash concatenated
        let hash: String = hashes
            .iter()
            .map(|h| {
                let end = std::cmp::min(10, h.len());
                &h[..end]
            })
            .collect();

        // Constraint suffix: 2-digit 1-based ordinal for non-CLASS/MIRROR/RANDOM constraints
        let mut constraint_suffix = String::new();
        for c in constraint {
            if *c != CourseDataConstraint::Class
                && *c != CourseDataConstraint::Mirror
                && *c != CourseDataConstraint::Random
            {
                let ordinal = Self::constraint_ordinal(c);
                constraint_suffix.push_str(&format!("{:02}", ordinal + 1));
            }
        }

        let sep = std::path::MAIN_SEPARATOR;
        let prefix = if ln {
            REPLAY.get(lnmode as usize).copied().unwrap_or("")
        } else {
            ""
        };
        let constraint_part = if constraint_suffix.is_empty() {
            String::new()
        } else {
            format!("_{}", constraint_suffix)
        };
        let index_part = if index > 0 {
            format!("_{}", index)
        } else {
            String::new()
        };
        format!(
            "{}{}{}{}{}{}",
            self.get_replay_data_folder(),
            sep,
            prefix,
            hash,
            constraint_part,
            index_part
        )
    }

    fn course_hash(models: &[BMSModel]) -> String {
        models
            .iter()
            .map(|m| m.sha256.as_str())
            .collect::<Vec<_>>()
            .join("")
    }

    pub(super) fn constraint_ordinal(c: &CourseDataConstraint) -> i32 {
        match c {
            CourseDataConstraint::Class => 0,
            CourseDataConstraint::Mirror => 1,
            CourseDataConstraint::Random => 2,
            CourseDataConstraint::NoSpeed => 3,
            CourseDataConstraint::NoGood => 4,
            CourseDataConstraint::NoGreat => 5,
            CourseDataConstraint::GaugeLr2 => 6,
            CourseDataConstraint::Gauge5Keys => 7,
            CourseDataConstraint::Gauge7Keys => 8,
            CourseDataConstraint::Gauge9Keys => 9,
            CourseDataConstraint::Gauge24Keys => 10,
            CourseDataConstraint::Ln => 11,
            CourseDataConstraint::Cn => 12,
            CourseDataConstraint::Hcn => 13,
        }
    }
}

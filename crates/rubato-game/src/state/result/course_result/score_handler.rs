use log::{info, warn};
use std::sync::Arc;

use crate::core::score_data::ScoreData;

use crate::state::result::{IRConnection, IRCourseData, IRScoreData};

pub(super) struct CourseIRSendStatus {
    pub ir: Arc<dyn IRConnection + Send + Sync>,
    pub course: crate::core::course_data::CourseData,
    pub lnmode: i32,
    pub score: ScoreData,
    pub retry: i32,
}

impl CourseIRSendStatus {
    pub fn new(
        ir: Arc<dyn IRConnection + Send + Sync>,
        course: &crate::core::course_data::CourseData,
        lnmode: i32,
        score: &ScoreData,
    ) -> Self {
        Self {
            ir,
            course: course.clone(),
            lnmode,
            score: score.clone(),
            retry: 0,
        }
    }

    pub fn send(&mut self) -> bool {
        info!("IR score sending: {:?}", self.course.name);
        let ir_course_data = IRCourseData::new_with_lntype(&self.course, self.lnmode);
        let ir_score_data = IRScoreData::new(&self.score);
        let send_result = self
            .ir
            .send_course_play_data(&ir_course_data, &ir_score_data);
        if send_result.is_succeeded() {
            info!("IR score send complete: {:?}", self.course.name);
            self.retry = -255;
            true
        } else {
            warn!("IR score send failed: {}", send_result.message);
            self.retry += 1;
            false
        }
    }
}

// ============================================================
// Pure computation helpers extracted from update_score_database
// ============================================================

/// Compute average judge timing when notes > 0.
/// Returns `total_duration / notes`, or 0 when `notes == 0`.
#[inline]
pub(super) fn compute_avgjudge(total_duration: i64, notes: i32) -> i64 {
    if notes == 0 {
        return 0;
    }
    total_duration / notes as i64
}

/// Apply avgjudge to a ScoreData in-place, guarding against division by zero.
/// When notes == 0, avgjudge is left unchanged (keeps its default of i64::MAX).
#[inline]
pub(super) fn apply_avgjudge(score: &mut crate::core::score_data::ScoreData) {
    if score.notes != 0 {
        score.timing_stats.avgjudge =
            compute_avgjudge(score.timing_stats.total_duration, score.notes);
    }
}

/// Determine the random mode value based on player config options and double-play flag.
///
/// Logic (translated from Java):
/// - Start with random = 0
/// - If random_cfg > 0 OR (dp AND (random2_cfg > 0 OR doubleoption_cfg > 0)): random = 2
/// - If random_cfg == 1 AND (!dp OR (random2_cfg == 1 AND doubleoption_cfg == 1)): random = 1
pub(super) fn determine_random_mode(
    random_cfg: i32,
    random2_cfg: i32,
    doubleoption_cfg: i32,
    dp: bool,
) -> i32 {
    let mut random = 0;
    if random_cfg > 0 || (dp && (random2_cfg > 0 || doubleoption_cfg > 0)) {
        random = 2;
    }
    if random_cfg == 1 && (!dp || (random2_cfg == 1 && doubleoption_cfg == 1)) {
        random = 1;
    }
    random
}

/// Check if any course BMS model uses double-play mode (player count == 2).
pub(super) fn is_double_play(models: &[bms::model::bms_model::BMSModel]) -> bool {
    models
        .iter()
        .any(|m| m.mode().map(|mode| mode.player()).unwrap_or(1) == 2)
}

/// Sum total notes across all course BMS models.
pub(super) fn aggregate_total_notes(models: &[bms::model::bms_model::BMSModel]) -> i32 {
    models.iter().map(|m| m.total_notes()).sum()
}

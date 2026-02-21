// CourseResult-specific skin state synchronization.
//
// Updates SharedGameState with aggregated course score and stage information.

use bms_skin::property_id::{
    OPTION_COURSE_STAGE_FINAL, OPTION_COURSE_STAGE1, OPTION_COURSE_STAGE2, OPTION_COURSE_STAGE3,
    OPTION_COURSE_STAGE4, OPTION_MODE_COURSE,
};

use crate::game_state::SharedGameState;
use crate::player_resource::PlayerResource;

/// Synchronize course result state into SharedGameState for skin rendering.
///
/// Calls sync_result_state for the aggregated score, then adds course-specific flags.
pub fn sync_course_result_state(state: &mut SharedGameState, resource: &PlayerResource) {
    // First sync the basic result properties using the aggregated score
    super::result_skin_state::sync_result_state(
        state,
        &resource.score_data,
        &resource.oldscore,
        resource.maxcombo,
        resource.target_exscore,
        None, // Course result doesn't show individual song metadata
    );

    // Course mode flag
    state.booleans.insert(OPTION_MODE_COURSE, true);

    // Stage flags
    let total = resource.course_total();
    state.booleans.insert(OPTION_COURSE_STAGE1, total >= 1);
    state.booleans.insert(OPTION_COURSE_STAGE2, total >= 2);
    state.booleans.insert(OPTION_COURSE_STAGE3, total >= 3);
    state.booleans.insert(OPTION_COURSE_STAGE4, total >= 4);
    state.booleans.insert(OPTION_COURSE_STAGE_FINAL, total > 0);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sync_course_result_sets_course_mode() {
        let mut state = SharedGameState::default();
        let resource = PlayerResource::default();

        sync_course_result_state(&mut state, &resource);

        assert!(*state.booleans.get(&OPTION_MODE_COURSE).unwrap());
    }

    #[test]
    fn sync_course_result_stage_flags() {
        let mut state = SharedGameState::default();
        let mut resource = PlayerResource::default();
        // Simulate a 3-stage course
        resource.course_bms_models = Some(vec![
            bms_model::BmsModel::default(),
            bms_model::BmsModel::default(),
            bms_model::BmsModel::default(),
        ]);

        sync_course_result_state(&mut state, &resource);

        assert!(*state.booleans.get(&OPTION_COURSE_STAGE1).unwrap());
        assert!(*state.booleans.get(&OPTION_COURSE_STAGE2).unwrap());
        assert!(*state.booleans.get(&OPTION_COURSE_STAGE3).unwrap());
        assert!(!*state.booleans.get(&OPTION_COURSE_STAGE4).unwrap());
        assert!(*state.booleans.get(&OPTION_COURSE_STAGE_FINAL).unwrap());
    }

    #[test]
    fn sync_course_result_includes_result_properties() {
        let mut state = SharedGameState::default();
        let mut resource = PlayerResource::default();
        resource.score_data.notes = 100;
        resource.score_data.epg = 50;

        sync_course_result_state(&mut state, &resource);

        // Should have result properties from sync_result_state
        assert!(
            state
                .integers
                .contains_key(&bms_skin::property_id::NUMBER_SCORE2)
        );
        assert!(
            state
                .booleans
                .contains_key(&bms_skin::property_id::OPTION_RESULT_CLEAR)
        );
    }
}

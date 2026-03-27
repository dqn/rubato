// Re-export from rubato-types (canonical location)
pub use rubato_types::score_data_property::ScoreDataProperty;

#[cfg(test)]
mod tests {
    use super::*;
    use bms::model::mode::Mode;
    use rubato_types::score_data::ScoreData;

    #[test]
    fn qualify_rank_out_of_bounds_returns_false() {
        let prop = ScoreDataProperty::default();
        // Array is [bool; 27], so index 27 is out of bounds.
        assert!(!prop.qualify_rank(27));
        assert!(!prop.qualify_rank(100));
        assert!(!prop.qualify_rank(usize::MAX));
    }

    #[test]
    fn qualify_now_rank_out_of_bounds_returns_false() {
        let prop = ScoreDataProperty::default();
        assert!(!prop.qualify_now_rank(27));
        assert!(!prop.qualify_now_rank(100));
    }

    #[test]
    fn qualify_best_rank_out_of_bounds_returns_false() {
        let prop = ScoreDataProperty::default();
        assert!(!prop.qualify_best_rank(27));
        assert!(!prop.qualify_best_rank(100));
    }

    #[test]
    fn qualify_rank_valid_index_returns_value() {
        let mut prop = ScoreDataProperty::default();
        prop.rank[18] = true;
        assert!(prop.qualify_rank(18));
        assert!(!prop.qualify_rank(17));
    }

    /// set_target_score_with_ghost must reset accumulated ghost state
    /// so that practice retries do not carry stale nowbestscore /
    /// nowrivalscore / previous_notes from the prior attempt.
    #[test]
    fn set_target_score_with_ghost_resets_stale_state() {
        let mut prop = ScoreDataProperty::default();

        // Simulate a first play: set ghost and accumulate scores
        let ghost = vec![0, 0, 1, 1, 2]; // PG, PG, GR, GR, GD
        prop.set_target_score_with_ghost(8, Some(ghost.clone()), 6, Some(ghost.clone()), 5);

        // Simulate updating scores during play to accumulate state
        let mut sd = ScoreData::new(Mode::BEAT_7K);
        sd.judge_counts.epg = 3;
        sd.notes = 5;
        prop.update_score_with_notes(Some(&sd), 3);

        // After partial play, these should be non-zero
        assert!(prop.nowbestscore > 0 || prop.nowrivalscore > 0 || prop.previous_notes > 0);

        // Now simulate a practice retry: call set_target_score_with_ghost again
        prop.set_target_score_with_ghost(8, Some(ghost.clone()), 6, Some(ghost), 5);

        // Accumulated state must be reset
        assert_eq!(
            prop.nowbestscore, 0,
            "nowbestscore should be reset on retry"
        );
        assert_eq!(
            prop.nowrivalscore, 0,
            "nowrivalscore should be reset on retry"
        );
        assert_eq!(
            prop.previous_notes, 0,
            "previous_notes should be reset on retry"
        );
    }
}

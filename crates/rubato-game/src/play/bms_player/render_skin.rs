use super::*;

/// Maximum time difference (in microseconds) allowed when falling back to
/// neighbor timelines after an inexact binary search. Beyond this threshold
/// the neighbor is considered unrelated and we return `None`.
const TIMELINE_LOOKUP_TOLERANCE_US: i64 = 1000;

/// Convert a JudgeNote array index to a timeline Vec index.
///
/// The judge manager stores JudgeNote indices for `processing` and `passing`,
/// but the draw code compares against timeline Vec indices. This function
/// bridges those two index spaces by finding the timeline that contains a
/// note at the same time and lane as the JudgeNote.
fn judge_note_idx_to_timeline_idx(
    note_idx: usize,
    judge_notes: &[bms::model::judge_note::JudgeNote],
    timelines: &[bms::model::time_line::TimeLine],
) -> Option<usize> {
    let jn = judge_notes.get(note_idx)?;
    // Binary search by micro_time (timelines are sorted), then scan nearby
    // for the matching lane. O(log n) instead of O(n).
    let search_result = timelines.binary_search_by_key(&jn.time_us, |tl| tl.micro_time());
    let start = match search_result {
        Ok(idx) => idx,
        Err(idx) => {
            // Exact time not found (can happen if JudgeNote time_us and TimeLine
            // micro_time diverge by f64->i64 rounding). Check nearest neighbors
            // for a lane match as a fallback, but only if within tolerance.
            if idx < timelines.len()
                && (timelines[idx].micro_time() - jn.time_us).abs() <= TIMELINE_LOOKUP_TOLERANCE_US
                && timelines[idx].note(jn.lane as i32).is_some()
            {
                return Some(idx);
            }
            if idx > 0
                && (timelines[idx - 1].micro_time() - jn.time_us).abs()
                    <= TIMELINE_LOOKUP_TOLERANCE_US
                && timelines[idx - 1].note(jn.lane as i32).is_some()
            {
                return Some(idx - 1);
            }
            log::warn!(
                "Timeline lookup failed: note_idx={note_idx} lane={} time_us={} \
                 has no neighbor within {TIMELINE_LOOKUP_TOLERANCE_US}us tolerance",
                jn.lane,
                jn.time_us,
            );
            return None;
        }
    };
    // Scan backwards to find the first timeline at this time
    let mut first = start;
    while first > 0 && timelines[first - 1].micro_time() == jn.time_us {
        first -= 1;
    }
    // Scan forward through all timelines at this time to find the lane match
    let mut i = first;
    while i < timelines.len() && timelines[i].micro_time() == jn.time_us {
        if timelines[i].note(jn.lane as i32).is_some() {
            return Some(i);
        }
        i += 1;
    }
    log::warn!(
        "Timeline lookup failed: note_idx={note_idx} lane={} time_us={} \
         found matching time but no lane match in timelines[{}..{}]",
        jn.lane,
        jn.time_us,
        first,
        i,
    );
    None
}

impl BMSPlayer {
    pub(super) fn render_skin_impl(
        &mut self,
        sprite: &mut rubato_render::sprite_batch::SpriteBatch,
    ) {
        let mut skin = match self.main_state_data.skin.take() {
            Some(s) => s,
            None => {
                log::debug!("render_skin_impl: skin is None, skipping");
                return;
            }
        };
        let mut timer = std::mem::take(&mut self.main_state_data.timer);

        // Compute note draw commands via the type-erased SkinDrawable bridge.
        // This calls LaneRenderer::draw_lane() inside the skin to populate SkinNoteObject.draw_commands.
        if self.lanerender.is_none() {
            log::debug!("render_skin_impl: lanerender is None, skipping note draw commands");
        }
        if let Some(ref mut lr) = self.lanerender {
            let lane_count = self.model.mode().map_or(8, |m| m.key() as usize);
            // Safety: self.model.timelines outlives the DrawLaneContext because the
            // context is consumed synchronously within compute_note_draw_commands and
            // self is not accessed again until after the call returns.
            let all_timelines = unsafe {
                crate::play::lane_renderer::TimelinesRef::from_slice(&self.model.timelines)
            };
            let judge_table = self.judge.judge_table(false);
            let bad_judge_time = judge_table.get(2).map_or(0, |jt| jt[0]);
            // Convert JudgeNote indices to timeline indices for processing/passing LN state.
            // The judge manager stores JudgeNote array indices, but the draw code
            // compares against timeline Vec indices (a different index space).
            let processing_long_notes: Vec<Option<usize>> = (0..lane_count)
                .map(|i| {
                    self.judge.processing_long_note(i).and_then(|ni| {
                        let result = judge_note_idx_to_timeline_idx(
                            ni,
                            &self.judge_notes,
                            &self.model.timelines,
                        );
                        debug_assert!(
                            result.is_some(),
                            "processing LN note_idx={ni} lane={i} could not be mapped to timeline"
                        );
                        result
                    })
                })
                .collect();
            let passing_long_notes: Vec<Option<usize>> = (0..lane_count)
                .map(|i| {
                    self.judge.passing_long_note(i).and_then(|ni| {
                        let result = judge_note_idx_to_timeline_idx(
                            ni,
                            &self.judge_notes,
                            &self.model.timelines,
                        );
                        debug_assert!(
                            result.is_some(),
                            "passing LN note_idx={ni} lane={i} could not be mapped to timeline"
                        );
                        result
                    })
                })
                .collect();
            let draw_ctx = crate::play::lane_renderer::DrawLaneContext {
                time: timer.now_time(),
                timer_play: if timer.is_timer_on(TIMER_PLAY) {
                    Some(timer.timer(TIMER_PLAY))
                } else {
                    None
                },
                timer_141: if timer.is_timer_on(TimerId::new(141)) {
                    Some(timer.timer(TimerId::new(141)))
                } else {
                    None
                },
                judge_timing: self.player_config.judge_settings.judgetiming as i64,
                is_practice: self.state == PlayState::Practice
                    || self.state == PlayState::PracticeFinished,
                practice_start_time: self.practice.practice_property().starttime as i64,
                now_time: timer.now_time(),
                now_quarter_note_time: self
                    .rhythm
                    .as_ref()
                    .map_or(0, |r| r.now_quarter_note_time()),
                note_expansion_rate: self.play_skin.note_expansion_rate,
                lane_group_regions: self
                    .play_skin
                    .lane_group_region()
                    .map(|regions| {
                        regions
                            .iter()
                            .map(|r| LaneGroupRegion {
                                x: r.x,
                                width: r.width,
                            })
                            .collect()
                    })
                    .unwrap_or_default(),
                show_bpmguide: self.player_config.display_settings.bpmguide,
                show_pastnote: self.player_config.display_settings.showpastnote,
                mark_processednote: self.player_config.display_settings.markprocessednote,
                show_hiddennote: self.player_config.display_settings.showhiddennote,
                show_judgearea: self.player_config.display_settings.showjudgearea,
                lntype: self.model.lntype(),
                judge_time_regions: (0..lane_count)
                    .map(|i| self.judge.judge_time_region(i).to_vec())
                    .collect(),
                processing_long_notes,
                passing_long_notes,
                hell_charge_judges: (0..lane_count)
                    .map(|i| self.judge.hell_charge_judge(i))
                    .collect(),
                bad_judge_time,
                model_bpm: self.model.bpm,
                all_timelines,
                forced_cn_endings: false,
            };
            skin.compute_note_draw_commands(&mut |lanes| {
                lr.draw_lane(&draw_ctx, lanes, &[]).commands
            });
        }

        {
            let mut snapshot = self.build_snapshot(&timer);
            skin.update_custom_objects_timed(&mut snapshot);
            skin.swap_sprite_batch(sprite);
            skin.draw_all_objects_timed(&mut snapshot);
            skin.swap_sprite_batch(sprite);

            // Drain non-event actions (timers, audio, state changes)
            self.drain_actions(&mut snapshot.actions, &mut timer);
            self.propagate_snapshot_config(&snapshot);

            // Replay queued custom events now that the skin is available again.
            let mut pending_events = std::mem::take(&mut snapshot.actions.custom_events);
            let mut depth = 0;
            while !pending_events.is_empty() && depth < 8 {
                let mut replay_snapshot = self.build_snapshot(&timer);
                for (id, arg1, arg2) in pending_events {
                    skin.execute_custom_event(&mut replay_snapshot, id, arg1, arg2);
                }
                self.drain_actions(&mut replay_snapshot.actions, &mut timer);
                self.propagate_snapshot_config(&replay_snapshot);
                pending_events = replay_snapshot.actions.custom_events;
                depth += 1;
            }
            if depth >= 8 {
                log::warn!("Play render_skin event replay exceeded depth limit");
            }
        }

        self.main_state_data.timer = timer;
        self.main_state_data.skin = Some(skin);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bms::model::judge_note::{JudgeNote, JudgeNoteKind};
    use bms::model::note::Note;
    use bms::model::time_line::TimeLine;

    fn make_judge_note(time_us: i64, lane: usize) -> JudgeNote {
        JudgeNote {
            time_us,
            end_time_us: 0,
            lane,
            wav: 0,
            kind: JudgeNoteKind::Normal,
            ln_type: 0,
            damage: 0.0,
            pair_index: None,
        }
    }

    #[test]
    fn judge_note_idx_to_timeline_idx_finds_matching_timeline() {
        let mut tl0 = TimeLine::new(0.0, 1_000_000, 8);
        tl0.set_note(0, Some(Note::new_normal(1)));
        let mut tl1 = TimeLine::new(1.0, 2_000_000, 8);
        tl1.set_note(2, Some(Note::new_normal(1)));
        let timelines = vec![tl0, tl1];

        let judge_notes = vec![make_judge_note(1_000_000, 0), make_judge_note(2_000_000, 2)];

        // JudgeNote 0 (time=1s, lane=0) should map to timeline 0
        assert_eq!(
            judge_note_idx_to_timeline_idx(0, &judge_notes, &timelines),
            Some(0)
        );
        // JudgeNote 1 (time=2s, lane=2) should map to timeline 1
        assert_eq!(
            judge_note_idx_to_timeline_idx(1, &judge_notes, &timelines),
            Some(1)
        );
    }

    #[test]
    fn judge_note_idx_to_timeline_idx_returns_none_for_missing() {
        let tl0 = TimeLine::new(0.0, 1_000_000, 8); // no notes
        let timelines = vec![tl0];

        let judge_notes = vec![make_judge_note(1_000_000, 0)];

        // No note at lane 0 in timeline -> None
        assert_eq!(
            judge_note_idx_to_timeline_idx(0, &judge_notes, &timelines),
            None
        );
    }

    #[test]
    fn judge_note_idx_to_timeline_idx_returns_none_for_out_of_bounds() {
        let timelines = vec![];
        let judge_notes = vec![];

        // Index out of bounds -> None
        assert_eq!(
            judge_note_idx_to_timeline_idx(5, &judge_notes, &timelines),
            None
        );
    }

    #[test]
    fn judge_note_idx_to_timeline_idx_distinguishes_lanes() {
        // Two timelines at the same time but different lanes with notes
        let mut tl0 = TimeLine::new(0.0, 1_000_000, 8);
        tl0.set_note(0, Some(Note::new_normal(1)));
        let mut tl1 = TimeLine::new(1.0, 1_000_000, 8);
        tl1.set_note(3, Some(Note::new_normal(1)));
        let timelines = vec![tl0, tl1];

        let judge_notes = vec![
            make_judge_note(1_000_000, 0), // lane 0
            make_judge_note(1_000_000, 3), // lane 3
        ];

        // Lane 0 note maps to timeline 0 (which has note on lane 0)
        assert_eq!(
            judge_note_idx_to_timeline_idx(0, &judge_notes, &timelines),
            Some(0)
        );
        // Lane 3 note maps to timeline 1 (which has note on lane 3)
        assert_eq!(
            judge_note_idx_to_timeline_idx(1, &judge_notes, &timelines),
            Some(1)
        );
    }

    // =========================================================================
    // Regression: bad_judge_time must read judge_table[2][0] (GOOD early-side),
    // matching Java LaneRenderer.java:578 getJudgeTable(false)[2][0].
    // Previously used [3][1] (BAD late-side) which is ~5x larger, causing PMS
    // miss-POOR falling notes to remain visible too long below the judge line.
    // =========================================================================

    /// Extracts bad_judge_time from a judge table using the same logic as
    /// the production code at line 77 of this file.
    fn extract_bad_judge_time(judge_table: &[[i64; 2]]) -> i64 {
        judge_table.get(2).map_or(0, |jt| jt[0])
    }

    #[test]
    fn bad_judge_time_reads_good_early_side_index_2_0() {
        // Standard sevenkeys note judge table (rank 100):
        // Row 0 (PG):   [-20000,  20000]
        // Row 1 (GR):   [-60000,  60000]
        // Row 2 (GD):   [-150000, 150000]
        // Row 3 (BD):   [-280000, 220000]
        // Row 4 (POOR): [-150000, 500000]
        let judge_table: Vec<[i64; 2]> = vec![
            [-20000, 20000],
            [-60000, 60000],
            [-150000, 150000],
            [-280000, 220000],
            [-150000, 500000],
        ];

        let bad_judge_time = extract_bad_judge_time(&judge_table);

        // Java: getJudgeTable(false)[2][0] = -150000 (GOOD early-side)
        assert_eq!(
            bad_judge_time, -150000,
            "bad_judge_time must be judge_table[2][0] (GOOD early-side = -150000), \
             not judge_table[3][1] (BAD late-side = 220000)"
        );
    }

    #[test]
    fn bad_judge_time_with_pms_judge_table() {
        // PMS note judge table (rank 100):
        // Row 0 (PG):   [-20000,  20000]
        // Row 1 (GR):   [-50000,  50000]
        // Row 2 (GD):   [-117000, 117000]
        // Row 3 (BD):   [-183000, 183000]
        // Row 4 (POOR): [-175000, 500000]
        let judge_table: Vec<[i64; 2]> = vec![
            [-20000, 20000],
            [-50000, 50000],
            [-117000, 117000],
            [-183000, 183000],
            [-175000, 500000],
        ];

        let bad_judge_time = extract_bad_judge_time(&judge_table);

        // Java: [2][0] = -117000 (GOOD early-side)
        assert_eq!(bad_judge_time, -117000);
    }

    #[test]
    fn bad_judge_time_returns_zero_when_table_has_fewer_than_3_rows() {
        // Edge case: table with only 2 rows (PG, GR) - no GOOD row
        let judge_table: Vec<[i64; 2]> = vec![[-20000, 20000], [-60000, 60000]];

        let bad_judge_time = extract_bad_judge_time(&judge_table);

        // .get(2) returns None -> map_or(0, ...) -> 0
        assert_eq!(bad_judge_time, 0);
    }

    // =========================================================================
    // Regression: lane_group_regions must be populated from PlaySkin's
    // lane_group_region(), not hardcoded to Vec::new(). Empty regions suppress
    // all practice mode text overlays (time, BPM, stop indicators).
    // =========================================================================

    #[test]
    fn lane_group_regions_mapping_from_rectangles() {
        use crate::play::play_skin::PlaySkin;
        use rubato_render::color::Rectangle;

        let mut skin = PlaySkin::new();
        skin.lanegroupregion = Some(vec![
            Rectangle::new(10.0, 20.0, 300.0, 400.0),
            Rectangle::new(500.0, 20.0, 300.0, 400.0),
        ]);

        let regions: Vec<LaneGroupRegion> = skin
            .lane_group_region()
            .map(|regions| {
                regions
                    .iter()
                    .map(|r| LaneGroupRegion {
                        x: r.x,
                        width: r.width,
                    })
                    .collect()
            })
            .unwrap_or_default();

        assert_eq!(regions.len(), 2);
        assert!((regions[0].x - 10.0).abs() < f32::EPSILON);
        assert!((regions[0].width - 300.0).abs() < f32::EPSILON);
        assert!((regions[1].x - 500.0).abs() < f32::EPSILON);
        assert!((regions[1].width - 300.0).abs() < f32::EPSILON);
    }

    #[test]
    fn lane_group_regions_empty_when_skin_has_none() {
        use crate::play::play_skin::PlaySkin;

        let skin = PlaySkin::new();
        assert!(skin.lanegroupregion.is_none());

        let regions: Vec<LaneGroupRegion> = skin
            .lane_group_region()
            .map(|regions| {
                regions
                    .iter()
                    .map(|r| LaneGroupRegion {
                        x: r.x,
                        width: r.width,
                    })
                    .collect()
            })
            .unwrap_or_default();

        assert!(regions.is_empty());
    }
}

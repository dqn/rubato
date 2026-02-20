// RhythmTimerProcessor — ported from Java RhythmTimerProcessor.java.
//
// Manages rhythm timers for section lines (measure boundaries) and
// quarter-note timing used for PMS rhythm note expansion.
// All timing uses integer microseconds (i64).

use std::collections::HashMap;

use bms_model::BmsModel;
use bms_skin::property_id::TIMER_RHYTHM;

use crate::timer_manager::TimerManager;

/// Processes rhythm timing for section lines and quarter-note beats.
///
/// Used during gameplay to drive `TIMER_RHYTHM` and track quarter-note
/// timing for PMS rhythm note expansion.
pub struct RhythmTimerProcessor {
    /// Microsecond times of each section line (measure boundary).
    section_times: Vec<i64>,
    /// Current section index (how many section lines have been passed).
    sections: usize,
    /// Current rhythm timer value (μs).
    rhythm_timer: i64,
    /// Microsecond times of each quarter-note beat (PMS only).
    quarter_note_times: Vec<i64>,
    /// Current quarter-note index.
    quarter_note: usize,
    /// Wall-clock millisecond time of the most recent quarter-note beat.
    now_quarter_note_time: i64,
}

impl RhythmTimerProcessor {
    /// Build a new RhythmTimerProcessor from the BMS model.
    ///
    /// When `use_quarter_note_time` is true (PMS/PopN9K mode), also computes
    /// quarter-note timing by subdividing sections at 0.25 intervals.
    pub fn new(model: &BmsModel, use_quarter_note_time: bool) -> Self {
        let timelines = &model.timelines;

        // Build a lookup for stop durations at each timeline time.
        let stop_at: HashMap<i64, i64> = model
            .stop_events
            .iter()
            .map(|s| (s.time_us, s.duration_us))
            .collect();

        let mut section_times = Vec::new();
        let mut quarter_note_times = Vec::new();

        for (i, tl) in timelines.iter().enumerate() {
            // Java: getSectionLine() — true when position == 0.0
            if tl.position != 0.0 {
                continue;
            }

            section_times.push(tl.time_us);

            if use_quarter_note_time {
                quarter_note_times.push(tl.time_us);

                // Java: sectionLineSection = timelines[i].getSection()
                let section_line_section = tl.measure as f64 + tl.position;

                // Find the next section line's section value (or last timeline's section).
                let mut next_section_line_section = 0.0;
                let mut last = false;
                for j in (i + 1)..timelines.len() {
                    if timelines[j].position == 0.0 {
                        next_section_line_section = (timelines[j].measure as f64
                            + timelines[j].position)
                            - section_line_section;
                        break;
                    } else if j == timelines.len() - 1 {
                        next_section_line_section = (timelines[j].measure as f64
                            + timelines[j].position)
                            - section_line_section;
                        last = true;
                    }
                }

                // Subdivide at 0.25 intervals between section lines.
                let mut j = 0.25;
                while j <= next_section_line_section {
                    if last || j != next_section_line_section {
                        // Find the timeline just before position j.
                        let mut prev_index = i;
                        while prev_index < timelines.len()
                            && (timelines[prev_index].measure as f64
                                + timelines[prev_index].position)
                                - section_line_section
                                < j
                        {
                            prev_index += 1;
                        }
                        prev_index = prev_index.saturating_sub(1);

                        let prev_tl = &timelines[prev_index];
                        let prev_section = prev_tl.measure as f64 + prev_tl.position;
                        let prev_stop = stop_at.get(&prev_tl.time_us).copied().unwrap_or(0);

                        let quarter_time = prev_tl.time_us
                            + prev_stop
                            + ((j + section_line_section - prev_section) * 240_000_000.0
                                / prev_tl.bpm) as i64;
                        quarter_note_times.push(quarter_time);
                    }
                    j += 0.25;
                }
            }
        }

        Self {
            section_times,
            sections: 0,
            rhythm_timer: 0,
            quarter_note_times,
            quarter_note: 0,
            now_quarter_note_time: 0,
        }
    }

    /// Update rhythm timer state for the current frame.
    ///
    /// # Arguments
    /// * `delta_time_us` - Frame delta time in microseconds.
    /// * `now_bpm` - Current BPM at this point in the chart.
    /// * `play_speed` - Play speed percentage (100 = normal).
    /// * `freq` - Frequency/speed multiplier (practice mode, 100 = normal).
    /// * `play_time_us` - Elapsed play time in microseconds (from TIMER_PLAY).
    /// * `now_time_ms` - Current wall-clock milliseconds (from timer.now_time()).
    /// * `timer` - Timer manager for setting TIMER_RHYTHM.
    #[allow(clippy::too_many_arguments)] // Matches Java interface; refactoring deferred
    pub fn update(
        &mut self,
        delta_time_us: i64,
        now_bpm: f64,
        play_speed: i32,
        freq: i32,
        play_time_us: i64,
        now_time_ms: i64,
        timer: &mut TimerManager,
    ) {
        // Java: rhythmtimer += deltatime * (100 - nowbpm * player.getPlaySpeed() / 60) / 100
        self.rhythm_timer +=
            delta_time_us * (100 - (now_bpm * play_speed as f64 / 60.0) as i64) / 100;
        timer.set_micro_timer(TIMER_RHYTHM, self.rhythm_timer);

        // Advance section when play_time reaches the next section line.
        // Java: sectiontimes[sections] * (100 / freq)
        let freq_scale = if freq > 0 { 100 / freq as i64 } else { 1 };
        if self.sections < self.section_times.len()
            && self.section_times[self.sections] * freq_scale <= play_time_us
        {
            self.sections += 1;
            timer.set_timer_on(TIMER_RHYTHM);
            self.rhythm_timer = timer.now_micro_time();
        }

        // Advance quarter-note tracking (PMS rhythm note expansion).
        if !self.quarter_note_times.is_empty() {
            if self.quarter_note < self.quarter_note_times.len()
                && self.quarter_note_times[self.quarter_note] * freq_scale <= play_time_us
            {
                self.quarter_note += 1;
                self.now_quarter_note_time = now_time_ms;
            } else if self.quarter_note == self.quarter_note_times.len()
                && freq_scale > 0
                && ((self.now_quarter_note_time + (60_000.0 / now_bpm) as i64) * freq_scale)
                    <= now_time_ms
            {
                self.now_quarter_note_time = now_time_ms;
            }
        }
    }

    /// Returns the wall-clock millisecond time of the most recent quarter-note beat.
    pub fn now_quarter_note_time(&self) -> i64 {
        self.now_quarter_note_time
    }

    /// Returns the section times (for testing).
    #[cfg(test)]
    pub fn section_times(&self) -> &[i64] {
        &self.section_times
    }

    /// Returns the quarter-note times (for testing).
    #[cfg(test)]
    pub fn quarter_note_times(&self) -> &[i64] {
        &self.quarter_note_times
    }

    /// Returns the current section index (for testing).
    #[cfg(test)]
    pub fn sections(&self) -> usize {
        self.sections
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bms_model::TimeLine;

    /// Build a minimal BmsModel with the given timelines for testing.
    fn model_with_timelines(timelines: Vec<TimeLine>) -> BmsModel {
        let mut model = BmsModel::default();
        model.timelines = timelines;
        model
    }

    #[test]
    fn fixed_bpm_120_section_times() {
        // 4 measures at BPM 120. Each measure = 60/120 * 4 = 2 seconds = 2_000_000 μs.
        let timelines = vec![
            TimeLine {
                time_us: 0,
                measure: 0,
                position: 0.0,
                bpm: 120.0,
                scroll: 1.0,
            },
            TimeLine {
                time_us: 500_000,
                measure: 0,
                position: 0.25,
                bpm: 120.0,
                scroll: 1.0,
            },
            TimeLine {
                time_us: 2_000_000,
                measure: 1,
                position: 0.0,
                bpm: 120.0,
                scroll: 1.0,
            },
            TimeLine {
                time_us: 4_000_000,
                measure: 2,
                position: 0.0,
                bpm: 120.0,
                scroll: 1.0,
            },
            TimeLine {
                time_us: 6_000_000,
                measure: 3,
                position: 0.0,
                bpm: 120.0,
                scroll: 1.0,
            },
        ];

        let model = model_with_timelines(timelines);
        let proc = RhythmTimerProcessor::new(&model, false);

        assert_eq!(proc.section_times(), &[0, 2_000_000, 4_000_000, 6_000_000]);
        assert!(proc.quarter_note_times().is_empty());
    }

    #[test]
    fn fixed_bpm_120_quarter_note_times() {
        // 2 measures at BPM 120. Each measure = 2_000_000 μs.
        // Quarter note at BPM 120 = 240_000_000 / 120 = 2_000_000 μs.
        // But subdividing section interval (1.0) at 0.25 gives 4 quarter notes.
        // Each 0.25 section at BPM 120 = 0.25 * 240_000_000 / 120 = 500_000 μs.
        let timelines = vec![
            TimeLine {
                time_us: 0,
                measure: 0,
                position: 0.0,
                bpm: 120.0,
                scroll: 1.0,
            },
            TimeLine {
                time_us: 2_000_000,
                measure: 1,
                position: 0.0,
                bpm: 120.0,
                scroll: 1.0,
            },
            TimeLine {
                time_us: 4_000_000,
                measure: 2,
                position: 0.0,
                bpm: 120.0,
                scroll: 1.0,
            },
        ];

        let model = model_with_timelines(timelines);
        let proc = RhythmTimerProcessor::new(&model, true);

        // Section times: [0, 2_000_000, 4_000_000]
        assert_eq!(proc.section_times(), &[0, 2_000_000, 4_000_000]);

        // Quarter note times for measure 0 (section 0.0, next section 1.0):
        //   j = 0.25: 0 + 0 + 0.25 * 240_000_000 / 120 = 500_000
        //   j = 0.50: 0 + 0 + 0.50 * 240_000_000 / 120 = 1_000_000
        //   j = 0.75: 0 + 0 + 0.75 * 240_000_000 / 120 = 1_500_000
        //   j = 1.00: excluded (j != nextSectionLineSection when !last)
        //
        // For measure 1 (section 1.0, next section 2.0):
        //   j = 0.25: 2_000_000 + 0 + 0.25 * 240_000_000 / 120 = 2_500_000
        //   j = 0.50: 2_000_000 + 0 + 0.50 * 240_000_000 / 120 = 3_000_000
        //   j = 0.75: 2_000_000 + 0 + 0.75 * 240_000_000 / 120 = 3_500_000
        //   j = 1.00: excluded
        //
        // For measure 2 (section 2.0, no next section line found):
        //   nextSectionLineSection = 0.0, loop body doesn't execute.
        let expected = vec![
            0,         // section 0 start
            500_000,   // 0.25
            1_000_000, // 0.50
            1_500_000, // 0.75
            2_000_000, // section 1 start
            2_500_000, // 1.25
            3_000_000, // 1.50
            3_500_000, // 1.75
            4_000_000, // section 2 start (no subdivisions — last section with no next)
        ];
        assert_eq!(proc.quarter_note_times(), &expected);
    }

    #[test]
    fn bpm_change_quarter_note_times() {
        // Measure 0 at BPM 120, measure 1 at BPM 240.
        // At BPM 120: 0.25 section = 0.25 * 240_000_000 / 120 = 500_000 μs.
        // At BPM 240: 0.25 section = 0.25 * 240_000_000 / 240 = 250_000 μs.
        let timelines = vec![
            TimeLine {
                time_us: 0,
                measure: 0,
                position: 0.0,
                bpm: 120.0,
                scroll: 1.0,
            },
            TimeLine {
                time_us: 2_000_000,
                measure: 1,
                position: 0.0,
                bpm: 240.0,
                scroll: 1.0,
            },
            // Last timeline: needed so measure 1 has content to subdivide
            TimeLine {
                time_us: 3_000_000,
                measure: 2,
                position: 0.0,
                bpm: 240.0,
                scroll: 1.0,
            },
        ];

        let model = model_with_timelines(timelines);
        let proc = RhythmTimerProcessor::new(&model, true);

        // Quarter note times for measure 0 (BPM 120, section 0.0 -> 1.0):
        //   0 (section start), 500_000, 1_000_000, 1_500_000
        // Quarter note times for measure 1 (BPM 240, section 1.0 -> 2.0):
        //   2_000_000 (section start), 2_250_000, 2_500_000, 2_750_000

        let qnt = proc.quarter_note_times();
        assert_eq!(qnt[0], 0);
        assert!((qnt[1] - 500_000).abs() <= 2);
        assert!((qnt[2] - 1_000_000).abs() <= 2);
        assert!((qnt[3] - 1_500_000).abs() <= 2);
        assert_eq!(qnt[4], 2_000_000);
        assert!((qnt[5] - 2_250_000).abs() <= 2);
        assert!((qnt[6] - 2_500_000).abs() <= 2);
        assert!((qnt[7] - 2_750_000).abs() <= 2);
    }

    #[test]
    fn update_advances_sections() {
        let timelines = vec![
            TimeLine {
                time_us: 0,
                measure: 0,
                position: 0.0,
                bpm: 120.0,
                scroll: 1.0,
            },
            TimeLine {
                time_us: 2_000_000,
                measure: 1,
                position: 0.0,
                bpm: 120.0,
                scroll: 1.0,
            },
            TimeLine {
                time_us: 4_000_000,
                measure: 2,
                position: 0.0,
                bpm: 120.0,
                scroll: 1.0,
            },
        ];

        let model = model_with_timelines(timelines);
        let mut proc = RhythmTimerProcessor::new(&model, false);
        let mut timer = TimerManager::new();

        // Before first section
        timer.set_micro_timer(TIMER_RHYTHM, 0);
        proc.update(16_000, 120.0, 100, 100, 0, 0, &mut timer);
        // Section 0 time is 0, so we should advance past it
        assert_eq!(proc.sections(), 1);

        // Advance to just before section 1 (2_000_000 μs)
        proc.update(16_000, 120.0, 100, 100, 1_999_999, 1999, &mut timer);
        assert_eq!(proc.sections(), 1);

        // Advance to section 1 time
        proc.update(16_000, 120.0, 100, 100, 2_000_000, 2000, &mut timer);
        assert_eq!(proc.sections(), 2);
    }

    #[test]
    fn update_quarter_note_progression() {
        let timelines = vec![
            TimeLine {
                time_us: 0,
                measure: 0,
                position: 0.0,
                bpm: 120.0,
                scroll: 1.0,
            },
            TimeLine {
                time_us: 2_000_000,
                measure: 1,
                position: 0.0,
                bpm: 120.0,
                scroll: 1.0,
            },
        ];

        let model = model_with_timelines(timelines);
        let mut proc = RhythmTimerProcessor::new(&model, true);
        let mut timer = TimerManager::new();

        // Quarter note times: [0, 500_000, 1_000_000, 1_500_000, 2_000_000]
        // Advance past the first quarter note (time 0)
        proc.update(16_000, 120.0, 100, 100, 0, 0, &mut timer);
        assert_eq!(proc.quarter_note, 1);

        // Advance to 500_000 μs
        proc.update(16_000, 120.0, 100, 100, 500_000, 500, &mut timer);
        assert_eq!(proc.quarter_note, 2);
        assert_eq!(proc.now_quarter_note_time(), 500);
    }
}

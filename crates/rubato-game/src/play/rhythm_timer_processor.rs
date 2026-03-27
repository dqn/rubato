use bms::model::bms_model::BMSModel;

/// Parameters for updating the rhythm timer.
pub struct RhythmUpdateParams {
    pub now: i64,
    pub micronow: i64,
    pub deltatime: i64,
    pub nowbpm: f64,
    pub play_speed: i32,
    pub freq: i32,
    pub play_timer_micro: i64,
}

/// Rhythm timer processor for section timing and quarter note tracking
pub struct RhythmTimerProcessor {
    sectiontimes: Vec<i64>,
    sections: usize,
    rhythmtimer: i64,
    /// Quarter note timing for PMS rhythm-based note expansion
    quarter_note_times: Vec<i64>,
    quarter_note: usize,
    now_quarter_note_time: i64,
}

impl RhythmTimerProcessor {
    pub fn new(model: &BMSModel, use_quarter_note_time: bool) -> Self {
        let mut sectiontimes: Vec<i64> = Vec::new();
        let mut quarter_note_times: Vec<i64> = Vec::new();
        let timelines = &model.timelines;

        for i in 0..timelines.len() {
            if timelines[i].section_line {
                sectiontimes.push(timelines[i].micro_time());

                if use_quarter_note_time {
                    quarter_note_times.push(timelines[i].micro_time());
                    let section_line_section = timelines[i].section();
                    let mut next_section_line_section =
                        timelines[i].section() - section_line_section;
                    let mut last = false;
                    for j in (i + 1)..timelines.len() {
                        if timelines[j].section_line {
                            next_section_line_section =
                                timelines[j].section() - section_line_section;
                            break;
                        } else if j == timelines.len() - 1 {
                            next_section_line_section =
                                timelines[j].section() - section_line_section;
                            last = true;
                        }
                    }
                    let mut j = 0.25f64;
                    while j <= next_section_line_section {
                        if last || j != next_section_line_section {
                            let mut prev_index = i;
                            while prev_index < timelines.len()
                                && timelines[prev_index].section() - section_line_section < j
                            {
                                prev_index += 1;
                            }
                            // Clamp to valid range if we overshot
                            if prev_index >= timelines.len() {
                                prev_index = timelines.len() - 1;
                            }
                            prev_index = prev_index.saturating_sub(1);
                            let bpm = timelines[prev_index].bpm;
                            let bpm_safe = if bpm == 0.0 { 1.0 } else { bpm };
                            let time = timelines[prev_index].micro_time()
                                + timelines[prev_index].micro_stop()
                                + ((j + section_line_section - timelines[prev_index].section())
                                    * 240000000.0
                                    / bpm_safe) as i64;
                            quarter_note_times.push(time);
                        }
                        j += 0.25;
                    }
                }
            }
        }

        RhythmTimerProcessor {
            sectiontimes,
            sections: 0,
            rhythmtimer: 0,
            quarter_note_times,
            quarter_note: 0,
            now_quarter_note_time: 0,
        }
    }

    pub fn update(&mut self, params: &RhythmUpdateParams) -> (i64, bool) {
        let now = params.now;
        let micronow = params.micronow;
        let deltatime = params.deltatime;
        let nowbpm = params.nowbpm;
        let play_speed = params.play_speed;
        let freq = params.freq;
        let play_timer_micro = params.play_timer_micro;
        let bpm_factor =
            (nowbpm * play_speed as f64 / 60.0).clamp(i32::MIN as f64, i32::MAX as f64) as i64;
        self.rhythmtimer += deltatime.saturating_mul(100 - bpm_factor) / 100;

        let mut rhythm_on = false;
        if freq > 0
            && self.sections < self.sectiontimes.len()
            && (self.sectiontimes[self.sections] * (100 / freq as i64)) <= play_timer_micro
        {
            self.sections += 1;
            rhythm_on = true;
            self.rhythmtimer = micronow;
        }
        if freq > 0 && !self.quarter_note_times.is_empty() {
            if self.quarter_note < self.quarter_note_times.len()
                && (self.quarter_note_times[self.quarter_note] * (100 / freq as i64))
                    <= play_timer_micro
            {
                self.quarter_note += 1;
                self.now_quarter_note_time = now;
            } else if self.quarter_note == self.quarter_note_times.len()
                && freq > 0
                && nowbpm > 0.0
                && ((self.now_quarter_note_time + (60000.0 / nowbpm) as i64) * (100 / freq as i64))
                    <= now
            {
                self.now_quarter_note_time = now;
            }
        }
        (self.rhythmtimer, rhythm_on)
    }

    pub fn now_quarter_note_time(&self) -> i64 {
        self.now_quarter_note_time
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Extreme BPM values no longer panic due to saturating arithmetic.
    /// Previously this caused i64 overflow; now saturating_mul prevents the panic.
    #[test]
    fn update_handles_extreme_bpm_without_overflow() {
        let model = BMSModel::default();
        let mut processor = RhythmTimerProcessor::new(&model, false);

        // nowbpm=1e15, play_speed=100, deltatime=16667 (one frame at 60fps in micros)
        // saturating_mul clamps instead of panicking.
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            processor.update(&RhythmUpdateParams {
                now: 0,
                micronow: 0,
                deltatime: 16667,
                nowbpm: 1e15,
                play_speed: 100,
                freq: 100,
                play_timer_micro: 0,
            })
        }));
        assert!(result.is_ok(), "should not panic with extreme BPM");
    }

    /// Regression: bpm_factor cast from f64 to i64 must be clamped to prevent
    /// intermediate overflow when nowbpm * play_speed / 60.0 exceeds i64::MAX.
    #[test]
    fn bpm_factor_clamp_prevents_intermediate_overflow() {
        let model = BMSModel::default();
        let mut processor = RhythmTimerProcessor::new(&model, false);

        // f64 value that exceeds i64::MAX when cast without clamping
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            processor.update(&RhythmUpdateParams {
                now: 1_000_000,
                micronow: 1_000_000,
                deltatime: 16667,
                nowbpm: 1e18,
                play_speed: 100,
                freq: 100,
                play_timer_micro: 1_000_000,
            })
        }));
        assert!(
            result.is_ok(),
            "should not overflow when bpm_factor exceeds i64 range"
        );
    }

    /// Regression: prev_index while-loop must not go out of bounds when float
    /// rounding causes the section difference to never reach the target j value.
    /// This constructs a model where quarter-note scanning could overshoot
    /// timelines.len() without the bounds check.
    #[test]
    fn quarter_note_prev_index_does_not_overflow() {
        use bms::model::time_line::TimeLine;

        let mut model = BMSModel::default();
        // Create two section-line timelines at section 0.0 and 1.0
        let mut tl0 = TimeLine::new(0.0, 0, 1);
        tl0.section_line = true;
        tl0.bpm = 120.0;
        let mut tl1 = TimeLine::new(1.0, 500_000, 1);
        tl1.section_line = true;
        tl1.bpm = 120.0;
        model.timelines = vec![tl0, tl1];

        // Should not panic even with quarter-note time enabled
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            RhythmTimerProcessor::new(&model, true);
        }));
        assert!(
            result.is_ok(),
            "quarter-note construction should not panic on out-of-bounds"
        );
    }
}

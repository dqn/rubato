use bms_model::bms_model::BMSModel;

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
        let timelines = model.get_all_time_lines();

        for i in 0..timelines.len() {
            if timelines[i].get_section_line() {
                sectiontimes.push(timelines[i].get_micro_time());

                if use_quarter_note_time {
                    quarter_note_times.push(timelines[i].get_micro_time());
                    let section_line_section = timelines[i].get_section();
                    let mut next_section_line_section =
                        timelines[i].get_section() - section_line_section;
                    let mut last = false;
                    for j in (i + 1)..timelines.len() {
                        if timelines[j].get_section_line() {
                            next_section_line_section =
                                timelines[j].get_section() - section_line_section;
                            break;
                        } else if j == timelines.len() - 1 {
                            next_section_line_section =
                                timelines[j].get_section() - section_line_section;
                            last = true;
                        }
                    }
                    let mut j = 0.25f64;
                    while j <= next_section_line_section {
                        if last || j != next_section_line_section {
                            let mut prev_index = i;
                            while timelines[prev_index].get_section() - section_line_section < j {
                                prev_index += 1;
                            }
                            prev_index -= 1;
                            let bpm = timelines[prev_index].get_bpm();
                            let bpm_safe = if bpm == 0.0 { 1.0 } else { bpm };
                            let time = timelines[prev_index].get_micro_time()
                                + timelines[prev_index].get_micro_stop()
                                + ((j + section_line_section - timelines[prev_index].get_section())
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

    #[allow(clippy::too_many_arguments)]
    pub fn update(
        &mut self,
        now: i64,
        micronow: i64,
        deltatime: i64,
        nowbpm: f64,
        play_speed: i32,
        freq: i32,
        play_timer_micro: i64,
    ) -> (i64, bool) {
        self.rhythmtimer +=
            deltatime.saturating_mul(100 - (nowbpm * play_speed as f64 / 60.0) as i64) / 100;

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

    pub fn get_now_quarter_note_time(&self) -> i64 {
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
            processor.update(
                0,     // now
                0,     // micronow
                16667, // deltatime (~16.6ms)
                1e15,  // nowbpm (extreme)
                100,   // play_speed
                100,   // freq
                0,     // play_timer_micro
            )
        }));
        assert!(result.is_ok(), "should not panic with extreme BPM");
    }
}

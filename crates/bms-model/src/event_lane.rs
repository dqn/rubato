use crate::bms_model::BMSModel;
use crate::time_line::TimeLine;

pub struct EventLane {
    sections: Vec<usize>,
    sectionbasepos: usize,
    sectionseekpos: usize,
    bpms: Vec<usize>,
    bpmbasepos: usize,
    bpmseekpos: usize,
    stops: Vec<usize>,
    stopbasepos: usize,
    stopseekpos: usize,
}

impl EventLane {
    pub fn new(model: &BMSModel) -> Self {
        let mut sections = Vec::new();
        let mut bpms = Vec::new();
        let mut stops = Vec::new();

        let timelines = &model.timelines;
        let mut prev_bpm: Option<f64> = None;
        for (i, tl) in timelines.iter().enumerate() {
            if tl.section_line {
                sections.push(i);
            }
            let compare_bpm = prev_bpm.unwrap_or(model.bpm);
            if tl.bpm != compare_bpm {
                bpms.push(i);
            }
            if tl.stop() != 0 {
                stops.push(i);
            }
            prev_bpm = Some(tl.bpm);
        }

        EventLane {
            sections,
            sectionbasepos: 0,
            sectionseekpos: 0,
            bpms,
            bpmbasepos: 0,
            bpmseekpos: 0,
            stops,
            stopbasepos: 0,
            stopseekpos: 0,
        }
    }

    pub fn sections(&self) -> &[usize] {
        &self.sections
    }

    pub fn bpm_changes(&self) -> &[usize] {
        &self.bpms
    }

    pub fn stops(&self) -> &[usize] {
        &self.stops
    }

    pub fn section(&mut self) -> Option<usize> {
        if self.sectionseekpos < self.sections.len() {
            let pos = self.sectionseekpos;
            self.sectionseekpos += 1;
            Some(self.sections[pos])
        } else {
            None
        }
    }

    pub fn bpm(&mut self) -> Option<usize> {
        if self.bpmseekpos < self.bpms.len() {
            let pos = self.bpmseekpos;
            self.bpmseekpos += 1;
            Some(self.bpms[pos])
        } else {
            None
        }
    }

    pub fn stop(&mut self) -> Option<usize> {
        if self.stopseekpos < self.stops.len() {
            let pos = self.stopseekpos;
            self.stopseekpos += 1;
            Some(self.stops[pos])
        } else {
            None
        }
    }

    pub fn reset(&mut self) {
        self.sectionseekpos = self.sectionbasepos;
        self.bpmseekpos = self.bpmbasepos;
        self.stopseekpos = self.stopbasepos;
    }

    pub fn mark(&mut self, time: i64, timelines: &[TimeLine]) {
        if !self.sections.is_empty() {
            while self.sectionbasepos < self.sections.len() - 1
                && timelines[self.sections[self.sectionbasepos + 1]].time() < time
            {
                self.sectionbasepos += 1;
            }
            while self.sectionbasepos > 0
                && timelines[self.sections[self.sectionbasepos]].time() > time
            {
                self.sectionbasepos -= 1;
            }
        }
        if !self.bpms.is_empty() {
            while self.bpmbasepos < self.bpms.len() - 1
                && timelines[self.bpms[self.bpmbasepos + 1]].time() < time
            {
                self.bpmbasepos += 1;
            }
            while self.bpmbasepos > 0 && timelines[self.bpms[self.bpmbasepos]].time() > time {
                self.bpmbasepos -= 1;
            }
        }
        if !self.stops.is_empty() {
            while self.stopbasepos < self.stops.len() - 1
                && timelines[self.stops[self.stopbasepos + 1]].time() < time
            {
                self.stopbasepos += 1;
            }
            while self.stopbasepos > 0 && timelines[self.stops[self.stopbasepos]].time() > time {
                self.stopbasepos -= 1;
            }
        }
        self.sectionseekpos = self.sectionbasepos;
        self.bpmseekpos = self.bpmbasepos;
        self.stopseekpos = self.stopbasepos;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_timelines(times_us: &[i64]) -> Vec<TimeLine> {
        times_us.iter().map(|&t| TimeLine::new(0.0, t, 0)).collect()
    }

    fn make_event_lane(section_indices: Vec<usize>) -> EventLane {
        EventLane {
            sections: section_indices,
            sectionbasepos: 0,
            sectionseekpos: 0,
            bpms: vec![],
            bpmbasepos: 0,
            bpmseekpos: 0,
            stops: vec![],
            stopbasepos: 0,
            stopseekpos: 0,
        }
    }

    #[test]
    fn mark_empty_sections_does_not_panic() {
        let timelines = make_timelines(&[]);
        let mut el = make_event_lane(vec![]);
        el.mark(1000, &timelines);
        assert_eq!(el.sectionbasepos, 0);
    }

    #[test]
    fn mark_seeks_forward() {
        // timelines at 1s, 2s, 3s (in microseconds)
        let timelines = make_timelines(&[1_000_000, 2_000_000, 3_000_000]);
        let mut el = make_event_lane(vec![0, 1, 2]);
        // time=2500 -> tl[1].time()=2000 < 2500, tl[2].time()=3000 >= 2500
        el.mark(2500, &timelines);
        assert_eq!(el.sectionbasepos, 1);
    }

    #[test]
    fn mark_seeks_backward() {
        let timelines = make_timelines(&[1_000_000, 2_000_000, 3_000_000]);
        let mut el = make_event_lane(vec![0, 1, 2]);
        // First seek forward to end
        el.mark(4000, &timelines);
        assert_eq!(el.sectionbasepos, 2);
        // Now seek backward to time=1500 -> tl[0].time()=1000 <= 1500
        el.mark(1500, &timelines);
        assert_eq!(el.sectionbasepos, 0);
    }
}

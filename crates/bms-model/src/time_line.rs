use crate::layer::Layer;
use crate::note::{Note, TYPE_CHARGENOTE, TYPE_HELLCHARGENOTE, TYPE_UNDEFINED};

#[derive(Clone)]
pub struct TimeLine {
    time: i64,
    section: f64,
    notes: Vec<Option<Note>>,
    hiddennotes: Vec<Option<Note>>,
    bgnotes: Vec<Note>,
    pub section_line: bool,
    pub bpm: f64,
    pub stop: i64,
    pub scroll: f64,
    pub bga: i32,
    pub layer: i32,
    pub eventlayer: Vec<Layer>,
}

impl TimeLine {
    pub fn new(section: f64, time: i64, notesize: i32) -> Self {
        let notesize = notesize as usize;
        let notes = vec![None; notesize];
        let hiddennotes = vec![None; notesize];
        TimeLine {
            section,
            time,
            notes,
            hiddennotes,
            bgnotes: Vec::new(),
            section_line: false,
            bpm: 0.0,
            stop: 0,
            scroll: 1.0,
            bga: -1,
            layer: -1,
            eventlayer: Vec::new(),
        }
    }

    pub fn time(&self) -> i64 {
        self.time / 1000
    }

    pub fn milli_time(&self) -> i64 {
        self.time / 1000
    }

    pub fn micro_time(&self) -> i64 {
        self.time
    }

    pub fn set_micro_time(&mut self, time: i64) {
        self.time = time;
        for note in self.notes.iter_mut().flatten() {
            note.set_micro_time(time);
        }
        for note in self.hiddennotes.iter_mut().flatten() {
            note.set_micro_time(time);
        }
        for n in &mut self.bgnotes {
            n.set_micro_time(time);
        }
    }

    pub fn lane_count(&self) -> i32 {
        self.notes.len() as i32
    }

    pub fn set_lane_count(&mut self, lanes: i32) {
        let lanes = lanes as usize;
        if self.notes.len() != lanes {
            let mut newnotes: Vec<Option<Note>> = Vec::with_capacity(lanes);
            let mut newhiddennotes: Vec<Option<Note>> = Vec::with_capacity(lanes);
            for i in 0..lanes {
                if i < self.notes.len() {
                    newnotes.push(self.notes[i].take());
                    newhiddennotes.push(self.hiddennotes[i].take());
                } else {
                    newnotes.push(None);
                    newhiddennotes.push(None);
                }
            }
            self.notes = newnotes;
            self.hiddennotes = newhiddennotes;
        }
    }

    pub fn total_notes(&self) -> i32 {
        self.total_notes_with_lntype(super::bms_model::LNTYPE_LONGNOTE)
    }

    pub fn total_notes_with_lntype(&self, lntype: super::bms_model::LnType) -> i32 {
        let mut count = 0;
        for note in self.notes.iter().flatten() {
            match note {
                Note::Long { note_type, end, .. } => {
                    if *note_type == TYPE_CHARGENOTE
                        || *note_type == TYPE_HELLCHARGENOTE
                        || (*note_type == TYPE_UNDEFINED
                            && lntype != super::bms_model::LNTYPE_LONGNOTE)
                        || !end
                    {
                        count += 1;
                    }
                }
                Note::Normal(_) => {
                    count += 1;
                }
                Note::Mine { .. } => {}
            }
        }
        count
    }

    pub fn exist_note(&self) -> bool {
        for n in &self.notes {
            if n.is_some() {
                return true;
            }
        }
        false
    }

    pub fn exist_note_at(&self, lane: i32) -> bool {
        let idx = lane as usize;
        idx < self.notes.len() && self.notes[idx].is_some()
    }

    pub fn note(&self, lane: i32) -> Option<&Note> {
        self.notes.get(lane as usize).and_then(|n| n.as_ref())
    }

    pub fn note_mut(&mut self, lane: i32) -> Option<&mut Note> {
        self.notes.get_mut(lane as usize).and_then(|n| n.as_mut())
    }

    pub fn set_note(&mut self, lane: i32, note: Option<Note>) {
        let lane = lane as usize;
        if let Some(mut n) = note {
            n.set_section(self.section);
            n.set_micro_time(self.time);
            self.notes[lane] = Some(n);
        } else {
            self.notes[lane] = None;
        }
    }

    pub fn set_hidden_note(&mut self, lane: i32, note: Option<Note>) {
        let lane = lane as usize;
        if let Some(mut n) = note {
            n.set_section(self.section);
            n.set_micro_time(self.time);
            self.hiddennotes[lane] = Some(n);
        } else {
            self.hiddennotes[lane] = None;
        }
    }

    pub fn exist_hidden_note(&self) -> bool {
        for n in &self.hiddennotes {
            if n.is_some() {
                return true;
            }
        }
        false
    }

    pub fn hidden_note(&self, lane: i32) -> Option<&Note> {
        self.hiddennotes.get(lane as usize).and_then(|n| n.as_ref())
    }

    pub fn add_back_ground_note(&mut self, note: Note) {
        let mut n = note;
        n.set_section(self.section);
        n.set_micro_time(self.time);
        self.bgnotes.push(n);
    }

    pub fn remove_back_ground_note(&mut self, index: usize) {
        if index < self.bgnotes.len() {
            self.bgnotes.remove(index);
        }
    }

    pub fn back_ground_notes(&self) -> &[Note] {
        &self.bgnotes
    }
    pub fn section(&self) -> f64 {
        self.section
    }

    pub fn set_section(&mut self, section: f64) {
        for note in self.notes.iter_mut().flatten() {
            note.set_section(section);
        }
        for note in self.hiddennotes.iter_mut().flatten() {
            note.set_section(section);
        }
        for n in &mut self.bgnotes {
            n.set_section(section);
        }
        self.section = section;
    }

    pub fn stop(&self) -> i32 {
        (self.stop / 1000) as i32
    }

    pub fn milli_stop(&self) -> i64 {
        self.stop / 1000
    }

    pub fn micro_stop(&self) -> i64 {
        self.stop
    }
    pub fn take_note(&mut self, lane: i32) -> Option<Note> {
        let idx = lane as usize;
        if idx < self.notes.len() {
            self.notes[idx].take()
        } else {
            None
        }
    }

    pub fn take_hidden_note(&mut self, lane: i32) -> Option<Note> {
        let idx = lane as usize;
        if idx < self.hiddennotes.len() {
            self.hiddennotes[idx].take()
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_creates_timeline_with_correct_values() {
        let tl = TimeLine::new(1.5, 2000, 8);
        assert!((tl.section() - 1.5).abs() < f64::EPSILON);
        assert_eq!(tl.micro_time(), 2000);
        assert_eq!(tl.milli_time(), 2);
        assert_eq!(tl.time(), 2);
        assert_eq!(tl.lane_count(), 8);
    }

    #[test]
    fn new_initializes_lanes_to_none() {
        let tl = TimeLine::new(0.0, 0, 4);
        for lane in 0..4 {
            assert!(!tl.exist_note_at(lane));
            assert!(tl.note(lane).is_none());
        }
        assert!(!tl.exist_note());
    }

    #[test]
    fn set_and_get_note() {
        let mut tl = TimeLine::new(1.0, 5000, 8);
        let note = Note::new_normal(42);
        tl.set_note(3, Some(note));

        assert!(tl.exist_note_at(3));
        assert!(tl.exist_note());
        let n = tl.note(3).unwrap();
        assert_eq!(n.wav(), 42);
        // Note should inherit section and time from the timeline
        assert!((n.section() - 1.0).abs() < f64::EPSILON);
        assert_eq!(n.micro_time(), 5000);
    }

    #[test]
    fn set_note_none_clears_lane() {
        let mut tl = TimeLine::new(0.0, 0, 4);
        tl.set_note(1, Some(Note::new_normal(1)));
        assert!(tl.exist_note_at(1));

        tl.set_note(1, None);
        assert!(!tl.exist_note_at(1));
    }

    #[test]
    fn set_and_get_hidden_note() {
        let mut tl = TimeLine::new(2.0, 10000, 8);
        assert!(!tl.exist_hidden_note());
        assert!(tl.hidden_note(0).is_none());

        let note = Note::new_normal(99);
        tl.set_hidden_note(0, Some(note));

        assert!(tl.exist_hidden_note());
        let n = tl.hidden_note(0).unwrap();
        assert_eq!(n.wav(), 99);
        assert!((n.section() - 2.0).abs() < f64::EPSILON);
        assert_eq!(n.micro_time(), 10000);
    }

    #[test]
    fn bpm_set_and_get() {
        let mut tl = TimeLine::new(0.0, 0, 8);
        assert!((tl.bpm).abs() < f64::EPSILON);

        tl.bpm = 150.0;
        assert!((tl.bpm - 150.0).abs() < f64::EPSILON);
    }

    #[test]
    fn section_line_set_and_get() {
        let mut tl = TimeLine::new(0.0, 0, 8);
        assert!(!tl.section_line);

        tl.section_line = true;
        assert!(tl.section_line);
    }

    #[test]
    fn bga_and_layer_set_and_get() {
        let mut tl = TimeLine::new(0.0, 0, 8);
        assert_eq!(tl.bga, -1);
        assert_eq!(tl.layer, -1);

        tl.bga = 5;
        tl.layer = 3;
        assert_eq!(tl.bga, 5);
        assert_eq!(tl.layer, 3);
    }

    #[test]
    fn stop_set_and_get() {
        let mut tl = TimeLine::new(0.0, 0, 8);
        assert_eq!(tl.micro_stop(), 0);

        tl.stop = 5000;
        assert_eq!(tl.micro_stop(), 5000);
        assert_eq!(tl.milli_stop(), 5);
        assert_eq!(tl.stop(), 5);
    }

    #[test]
    fn scroll_default_and_set() {
        let mut tl = TimeLine::new(0.0, 0, 8);
        assert!((tl.scroll - 1.0).abs() < f64::EPSILON);

        tl.scroll = 2.0;
        assert!((tl.scroll - 2.0).abs() < f64::EPSILON);
    }

    #[test]
    fn add_and_get_background_notes() {
        let mut tl = TimeLine::new(3.0, 6000, 8);
        assert!(tl.back_ground_notes().is_empty());

        let note = Note::new_normal(10);
        tl.add_back_ground_note(note);

        assert_eq!(tl.back_ground_notes().len(), 1);
        let bg = &tl.back_ground_notes()[0];
        assert_eq!(bg.wav(), 10);
        assert!((bg.section() - 3.0).abs() < f64::EPSILON);
        assert_eq!(bg.micro_time(), 6000);
    }

    #[test]
    fn remove_background_note() {
        let mut tl = TimeLine::new(0.0, 0, 8);
        tl.add_back_ground_note(Note::new_normal(1));
        tl.add_back_ground_note(Note::new_normal(2));
        assert_eq!(tl.back_ground_notes().len(), 2);

        tl.remove_back_ground_note(0);
        assert_eq!(tl.back_ground_notes().len(), 1);
        assert_eq!(tl.back_ground_notes()[0].wav(), 2);
    }

    #[test]
    fn remove_background_note_out_of_bounds_is_no_op() {
        let mut tl = TimeLine::new(0.0, 0, 8);
        tl.add_back_ground_note(Note::new_normal(1));
        tl.remove_back_ground_note(100);
        assert_eq!(tl.back_ground_notes().len(), 1);
    }

    #[test]
    fn set_micro_time_propagates_to_notes() {
        let mut tl = TimeLine::new(0.0, 0, 8);
        tl.set_note(0, Some(Note::new_normal(1)));
        tl.set_hidden_note(1, Some(Note::new_normal(2)));
        tl.add_back_ground_note(Note::new_normal(3));

        tl.set_micro_time(99000);

        assert_eq!(tl.micro_time(), 99000);
        assert_eq!(tl.note(0).unwrap().micro_time(), 99000);
        assert_eq!(tl.hidden_note(1).unwrap().micro_time(), 99000);
        assert_eq!(tl.back_ground_notes()[0].micro_time(), 99000);
    }

    #[test]
    fn set_section_propagates_to_notes() {
        let mut tl = TimeLine::new(0.0, 0, 8);
        tl.set_note(0, Some(Note::new_normal(1)));
        tl.set_hidden_note(1, Some(Note::new_normal(2)));
        tl.add_back_ground_note(Note::new_normal(3));

        tl.set_section(5.5);

        assert!((tl.section() - 5.5).abs() < f64::EPSILON);
        assert!((tl.note(0).unwrap().section() - 5.5).abs() < f64::EPSILON);
        assert!((tl.hidden_note(1).unwrap().section() - 5.5).abs() < f64::EPSILON);
        assert!((tl.back_ground_notes()[0].section() - 5.5).abs() < f64::EPSILON);
    }

    #[test]
    fn set_lane_count_expands() {
        let mut tl = TimeLine::new(0.0, 0, 4);
        assert_eq!(tl.lane_count(), 4);

        tl.set_lane_count(8);
        assert_eq!(tl.lane_count(), 8);
        // New lanes should be None
        for lane in 0..8 {
            assert!(!tl.exist_note_at(lane));
        }
    }

    #[test]
    fn set_lane_count_preserves_existing_notes() {
        let mut tl = TimeLine::new(0.0, 0, 4);
        tl.set_note(1, Some(Note::new_normal(42)));

        tl.set_lane_count(8);
        assert_eq!(tl.lane_count(), 8);
        assert_eq!(tl.note(1).unwrap().wav(), 42);
    }

    #[test]
    fn set_lane_count_shrinks() {
        let mut tl = TimeLine::new(0.0, 0, 8);
        tl.set_note(1, Some(Note::new_normal(42)));

        tl.set_lane_count(4);
        assert_eq!(tl.lane_count(), 4);
        assert_eq!(tl.note(1).unwrap().wav(), 42);
    }

    #[test]
    fn take_note_removes_and_returns() {
        let mut tl = TimeLine::new(0.0, 0, 8);
        tl.set_note(2, Some(Note::new_normal(77)));
        assert!(tl.exist_note_at(2));

        let taken = tl.take_note(2);
        assert!(taken.is_some());
        assert_eq!(taken.unwrap().wav(), 77);
        assert!(!tl.exist_note_at(2));
    }

    #[test]
    fn total_notes_counts_normal_and_long() {
        let mut tl = TimeLine::new(0.0, 0, 8);
        tl.set_note(0, Some(Note::new_normal(1)));
        tl.set_note(1, Some(Note::new_normal(2)));
        // Mine notes should not be counted
        tl.set_note(2, Some(Note::new_mine(3, 0.5)));

        assert_eq!(tl.total_notes(), 2);
    }

    #[test]
    fn eventlayer_set_and_get() {
        use crate::layer::{Event, EventType, Layer};

        let mut tl = TimeLine::new(0.0, 0, 8);
        assert!(tl.eventlayer.is_empty());

        let layers = vec![Layer::new(Event::new(EventType::Always, 0), vec![])];
        tl.eventlayer = layers;
        assert_eq!(tl.eventlayer.len(), 1);
    }

    #[test]
    fn time_returns_correct_value_beyond_i32_max() {
        // 36 minutes = 2_160_000 ms > i32::MAX (2_147_483_647 ms ~ 35.8 min)
        let micro_time: i64 = 2_160_000_000_000; // 36 min in microseconds
        let tl = TimeLine::new(0.0, micro_time, 8);
        assert_eq!(tl.time(), 2_160_000_000); // 36 min in milliseconds
        assert!(tl.time() > i32::MAX as i64);
    }
}

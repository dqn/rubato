use crate::bms_model::BMSModel;
use crate::note::Note;

pub struct Lane {
    notes: Vec<Note>,
    notebasepos: usize,
    noteseekpos: usize,
    hiddens: Vec<Note>,
    hiddenbasepos: usize,
    hiddenseekpos: usize,
}

impl Lane {
    pub fn new(model: &BMSModel, lane: i32) -> Self {
        let mut notes = Vec::new();
        let mut hiddens = Vec::new();
        for tl in &model.timelines {
            if tl.exist_note_at(lane)
                && let Some(note) = tl.note(lane)
            {
                notes.push(note.clone());
            }
            if let Some(hnote) = tl.hidden_note(lane) {
                hiddens.push(hnote.clone());
            }
        }
        Lane {
            notes,
            notebasepos: 0,
            noteseekpos: 0,
            hiddens,
            hiddenbasepos: 0,
            hiddenseekpos: 0,
        }
    }

    pub fn notes(&self) -> &[Note] {
        &self.notes
    }

    pub fn hiddens(&self) -> &[Note] {
        &self.hiddens
    }

    pub fn note(&mut self) -> Option<&Note> {
        if self.noteseekpos < self.notes.len() {
            let pos = self.noteseekpos;
            self.noteseekpos += 1;
            Some(&self.notes[pos])
        } else {
            None
        }
    }

    pub fn hidden(&mut self) -> Option<&Note> {
        if self.hiddenseekpos < self.hiddens.len() {
            let pos = self.hiddenseekpos;
            self.hiddenseekpos += 1;
            Some(&self.hiddens[pos])
        } else {
            None
        }
    }

    pub fn reset(&mut self) {
        self.noteseekpos = self.notebasepos;
        self.hiddenseekpos = self.hiddenbasepos;
    }

    pub fn mark(&mut self, time: i64) {
        if !self.notes.is_empty() {
            while self.notebasepos < self.notes.len() - 1
                && self.notes[self.notebasepos + 1].time() < time
            {
                self.notebasepos += 1;
            }
            while self.notebasepos > 0 && self.notes[self.notebasepos].time() > time {
                self.notebasepos -= 1;
            }
        }
        self.noteseekpos = self.notebasepos;
        if !self.hiddens.is_empty() {
            while self.hiddenbasepos < self.hiddens.len() - 1
                && self.hiddens[self.hiddenbasepos + 1].time() < time
            {
                self.hiddenbasepos += 1;
            }
            while self.hiddenbasepos > 0 && self.hiddens[self.hiddenbasepos].time() > time {
                self.hiddenbasepos -= 1;
            }
        }
        self.hiddenseekpos = self.hiddenbasepos;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::note::{Note, NoteData};

    fn make_note(time_us: i64) -> Note {
        Note::Normal(NoteData {
            time: time_us,
            ..NoteData::new()
        })
    }

    #[test]
    fn mark_empty_notes_does_not_panic() {
        let mut lane = Lane {
            notes: vec![],
            notebasepos: 0,
            noteseekpos: 0,
            hiddens: vec![],
            hiddenbasepos: 0,
            hiddenseekpos: 0,
        };
        lane.mark(1000);
        assert_eq!(lane.notebasepos, 0);
        assert_eq!(lane.hiddenbasepos, 0);
    }

    #[test]
    fn mark_empty_hiddens_with_notes() {
        let mut lane = Lane {
            notes: vec![make_note(2_000_000), make_note(4_000_000)],
            notebasepos: 0,
            noteseekpos: 0,
            hiddens: vec![],
            hiddenbasepos: 0,
            hiddenseekpos: 0,
        };
        // note[0].time()=2000 < 3000, but note[1].time()=4000 >= 3000
        // so notebasepos stays at 0 (next note not yet passed)
        lane.mark(3000);
        assert_eq!(lane.notebasepos, 0);
        assert_eq!(lane.hiddenbasepos, 0);
    }

    #[test]
    fn mark_seeks_forward_correctly() {
        let mut lane = Lane {
            notes: vec![
                make_note(1_000_000),
                make_note(2_000_000),
                make_note(3_000_000),
            ],
            notebasepos: 0,
            noteseekpos: 0,
            hiddens: vec![],
            hiddenbasepos: 0,
            hiddenseekpos: 0,
        };
        // time=2500 -> note[1].time()=2000 < 2500, note[2].time()=3000 >= 2500
        lane.mark(2500);
        assert_eq!(lane.notebasepos, 1);
    }
}

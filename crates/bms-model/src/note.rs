use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NoteData {
    pub section: f64,
    pub time: i64,
    pub wav: i32,
    pub start: i64,
    pub duration: i64,
    pub state: i32,
    pub playtime: i64,
    pub layerednotes: Vec<Note>,
}

impl Default for NoteData {
    fn default() -> Self {
        Self::new()
    }
}

impl NoteData {
    pub fn new() -> Self {
        NoteData {
            section: 0.0,
            time: 0,
            wav: 0,
            start: 0,
            duration: 0,
            state: 0,
            playtime: 0,
            layerednotes: Vec::new(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum Note {
    Normal(NoteData),
    Long {
        data: NoteData,
        end: bool,
        pair: Option<usize>,
        note_type: i32,
    },
    Mine {
        data: NoteData,
        damage: f64,
    },
}

pub const TYPE_UNDEFINED: i32 = 0;
pub const TYPE_LONGNOTE: i32 = 1;
pub const TYPE_CHARGENOTE: i32 = 2;
pub const TYPE_HELLCHARGENOTE: i32 = 3;

impl Note {
    pub fn new_normal(wav: i32) -> Self {
        let mut data = NoteData::new();
        data.wav = wav;
        Note::Normal(data)
    }

    pub fn new_normal_with_start_duration(wav: i32, start: i64, duration: i64) -> Self {
        let mut data = NoteData::new();
        data.wav = wav;
        data.start = start;
        data.duration = duration;
        Note::Normal(data)
    }

    pub fn new_long(wav: i32) -> Self {
        let mut data = NoteData::new();
        data.wav = wav;
        Note::Long {
            data,
            end: false,
            pair: None,
            note_type: TYPE_UNDEFINED,
        }
    }

    pub fn new_long_with_start_duration(wav: i32, starttime: i64, duration: i64) -> Self {
        let mut data = NoteData::new();
        data.wav = wav;
        data.start = starttime;
        data.duration = duration;
        Note::Long {
            data,
            end: false,
            pair: None,
            note_type: TYPE_UNDEFINED,
        }
    }

    pub fn new_mine(wav: i32, damage: f64) -> Self {
        let mut data = NoteData::new();
        data.wav = wav;
        Note::Mine { data, damage }
    }

    pub fn data(&self) -> &NoteData {
        match self {
            Note::Normal(data) => data,
            Note::Long { data, .. } => data,
            Note::Mine { data, .. } => data,
        }
    }

    pub fn data_mut(&mut self) -> &mut NoteData {
        match self {
            Note::Normal(data) => data,
            Note::Long { data, .. } => data,
            Note::Mine { data, .. } => data,
        }
    }

    pub fn wav(&self) -> i32 {
        self.data().wav
    }

    pub fn set_wav(&mut self, wav: i32) {
        self.data_mut().wav = wav;
    }

    pub fn state(&self) -> i32 {
        self.data().state
    }

    pub fn set_state(&mut self, state: i32) {
        self.data_mut().state = state;
    }

    pub fn milli_starttime(&self) -> i64 {
        self.data().start / 1000
    }

    pub fn micro_starttime(&self) -> i64 {
        self.data().start
    }

    pub fn set_micro_starttime(&mut self, start: i64) {
        self.data_mut().start = start;
    }

    pub fn milli_duration(&self) -> i64 {
        self.data().duration / 1000
    }

    pub fn micro_duration(&self) -> i64 {
        self.data().duration
    }

    pub fn set_micro_duration(&mut self, duration: i64) {
        self.data_mut().duration = duration;
    }

    pub fn play_time(&self) -> i64 {
        self.data().playtime / 1000
    }

    pub fn milli_play_time(&self) -> i64 {
        self.data().playtime / 1000
    }

    pub fn micro_play_time(&self) -> i64 {
        self.data().playtime
    }

    pub fn set_play_time(&mut self, playtime: i32) {
        self.data_mut().playtime = (playtime as i64) * 1000;
    }

    pub fn set_micro_play_time(&mut self, playtime: i64) {
        self.data_mut().playtime = playtime;
    }

    pub fn section(&self) -> f64 {
        self.data().section
    }

    pub fn set_section(&mut self, section: f64) {
        self.data_mut().section = section;
    }

    pub fn time(&self) -> i64 {
        self.data().time / 1000
    }

    pub fn milli_time(&self) -> i64 {
        self.data().time / 1000
    }

    pub fn micro_time(&self) -> i64 {
        self.data().time
    }

    pub fn set_micro_time(&mut self, time: i64) {
        self.data_mut().time = time;
    }

    pub fn add_layered_note(&mut self, mut n: Note) {
        let section = self.data().section;
        let time = self.data().time;
        n.set_section(section);
        n.set_micro_time(time);
        self.data_mut().layerednotes.push(n);
    }

    pub fn layered_notes(&self) -> &[Note] {
        &self.data().layerednotes
    }

    pub fn is_normal(&self) -> bool {
        matches!(self, Note::Normal(_))
    }

    pub fn is_long(&self) -> bool {
        matches!(self, Note::Long { .. })
    }

    pub fn is_mine(&self) -> bool {
        matches!(self, Note::Mine { .. })
    }

    pub fn long_note_type(&self) -> i32 {
        match self {
            Note::Long { note_type, .. } => *note_type,
            _ => TYPE_UNDEFINED,
        }
    }

    pub fn set_long_note_type(&mut self, t: i32) {
        if let Note::Long { note_type, .. } = self {
            *note_type = t;
        }
    }

    pub fn is_end(&self) -> bool {
        match self {
            Note::Long { end, .. } => *end,
            _ => false,
        }
    }

    pub fn set_end(&mut self, e: bool) {
        if let Note::Long { end, .. } = self {
            *end = e;
        }
    }

    pub fn pair(&self) -> Option<usize> {
        match self {
            Note::Long { pair, .. } => *pair,
            _ => None,
        }
    }

    pub fn set_pair_index(&mut self, idx: Option<usize>) {
        if let Note::Long { pair, .. } = self {
            *pair = idx;
        }
    }

    pub fn damage(&self) -> f64 {
        match self {
            Note::Mine { damage, .. } => *damage,
            _ => 0.0,
        }
    }

    pub fn set_damage(&mut self, d: f64) {
        if let Note::Mine { damage, .. } = self {
            *damage = d;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- NoteData tests ---

    #[test]
    fn note_data_new_defaults() {
        let data = NoteData::new();
        assert_eq!(data.section, 0.0);
        assert_eq!(data.time, 0);
        assert_eq!(data.wav, 0);
        assert_eq!(data.start, 0);
        assert_eq!(data.duration, 0);
        assert_eq!(data.state, 0);
        assert_eq!(data.playtime, 0);
        assert!(data.layerednotes.is_empty());
    }

    #[test]
    fn note_data_default_matches_new() {
        let from_new = NoteData::new();
        let from_default = NoteData::default();
        assert_eq!(from_new.wav, from_default.wav);
        assert_eq!(from_new.time, from_default.time);
        assert_eq!(from_new.section, from_default.section);
    }

    // --- Normal note tests ---

    #[test]
    fn new_normal_sets_wav() {
        let note = Note::new_normal(42);
        assert!(note.is_normal());
        assert!(!note.is_long());
        assert!(!note.is_mine());
        assert_eq!(note.wav(), 42);
    }

    #[test]
    fn new_normal_with_start_duration() {
        let note = Note::new_normal_with_start_duration(10, 5000, 3000);
        assert!(note.is_normal());
        assert_eq!(note.wav(), 10);
        assert_eq!(note.micro_starttime(), 5000);
        assert_eq!(note.milli_starttime(), 5);
        assert_eq!(note.micro_duration(), 3000);
        assert_eq!(note.milli_duration(), 3);
    }

    // --- LongNote tests ---

    #[test]
    fn new_long_sets_wav_and_defaults() {
        let note = Note::new_long(99);
        assert!(note.is_long());
        assert!(!note.is_normal());
        assert!(!note.is_mine());
        assert_eq!(note.wav(), 99);
        assert!(!note.is_end());
        assert_eq!(note.pair(), None);
        assert_eq!(note.long_note_type(), TYPE_UNDEFINED);
    }

    #[test]
    fn new_long_with_start_duration() {
        let note = Note::new_long_with_start_duration(7, 10000, 20000);
        assert!(note.is_long());
        assert_eq!(note.wav(), 7);
        assert_eq!(note.micro_starttime(), 10000);
        assert_eq!(note.micro_duration(), 20000);
    }

    #[test]
    fn long_note_pairing() {
        let mut start_note = Note::new_long(1);
        let mut end_note = Note::new_long(1);

        // Link the pair using indices
        start_note.set_pair_index(Some(1));
        end_note.set_pair_index(Some(0));
        end_note.set_end(true);

        assert_eq!(start_note.pair(), Some(1));
        assert!(!start_note.is_end());

        assert_eq!(end_note.pair(), Some(0));
        assert!(end_note.is_end());
    }

    #[test]
    fn long_note_type_set_and_get() {
        let mut note = Note::new_long(1);
        assert_eq!(note.long_note_type(), TYPE_UNDEFINED);

        note.set_long_note_type(TYPE_LONGNOTE);
        assert_eq!(note.long_note_type(), TYPE_LONGNOTE);

        note.set_long_note_type(TYPE_CHARGENOTE);
        assert_eq!(note.long_note_type(), TYPE_CHARGENOTE);

        note.set_long_note_type(TYPE_HELLCHARGENOTE);
        assert_eq!(note.long_note_type(), TYPE_HELLCHARGENOTE);
    }

    #[test]
    fn long_note_type_on_normal_returns_undefined() {
        let note = Note::new_normal(1);
        assert_eq!(note.long_note_type(), TYPE_UNDEFINED);
    }

    #[test]
    fn set_end_on_normal_is_no_op() {
        let mut note = Note::new_normal(1);
        note.set_end(true);
        assert!(!note.is_end());
    }

    #[test]
    fn set_pair_index_on_normal_is_no_op() {
        let mut note = Note::new_normal(1);
        note.set_pair_index(Some(5));
        assert_eq!(note.pair(), None);
    }

    // --- Mine note tests ---

    #[test]
    fn new_mine_sets_wav_and_damage() {
        let note = Note::new_mine(50, 0.5);
        assert!(note.is_mine());
        assert!(!note.is_normal());
        assert!(!note.is_long());
        assert_eq!(note.wav(), 50);
        assert!((note.damage() - 0.5).abs() < f64::EPSILON);
    }

    #[test]
    fn mine_damage_set_and_get() {
        let mut note = Note::new_mine(1, 0.1);
        assert!((note.damage() - 0.1).abs() < f64::EPSILON);

        note.set_damage(0.9);
        assert!((note.damage() - 0.9).abs() < f64::EPSILON);
    }

    #[test]
    fn get_damage_on_normal_returns_zero() {
        let note = Note::new_normal(1);
        assert!((note.damage()).abs() < f64::EPSILON);
    }

    // --- Shared accessor tests ---

    #[test]
    fn wav_set_and_get() {
        let mut note = Note::new_normal(1);
        note.set_wav(77);
        assert_eq!(note.wav(), 77);
    }

    #[test]
    fn state_set_and_get() {
        let mut note = Note::new_normal(1);
        assert_eq!(note.state(), 0);
        note.set_state(3);
        assert_eq!(note.state(), 3);
    }

    #[test]
    fn time_set_and_get() {
        let mut note = Note::new_normal(1);
        note.set_micro_time(123456);
        assert_eq!(note.micro_time(), 123456);
        assert_eq!(note.milli_time(), 123);
        assert_eq!(note.time(), 123);
    }

    #[test]
    fn section_set_and_get() {
        let mut note = Note::new_normal(1);
        note.set_section(2.5);
        assert!((note.section() - 2.5).abs() < f64::EPSILON);
    }

    #[test]
    fn play_time_set_and_get() {
        let mut note = Note::new_normal(1);
        note.set_play_time(500);
        assert_eq!(note.play_time(), 500);
        assert_eq!(note.milli_play_time(), 500);
        assert_eq!(note.micro_play_time(), 500_000);
    }

    #[test]
    fn micro_play_time_set_and_get() {
        let mut note = Note::new_normal(1);
        note.set_micro_play_time(999_999);
        assert_eq!(note.micro_play_time(), 999_999);
        assert_eq!(note.milli_play_time(), 999);
    }

    // --- Layered note tests ---

    #[test]
    fn add_and_get_layered_notes() {
        let mut note = Note::new_normal(1);
        note.set_section(1.0);
        note.set_micro_time(1000);
        assert!(note.layered_notes().is_empty());

        let layered = Note::new_normal(2);
        note.add_layered_note(layered);

        assert_eq!(note.layered_notes().len(), 1);
        let ln = &note.layered_notes()[0];
        assert_eq!(ln.wav(), 2);
        // Layered note inherits section and time from parent
        assert!((ln.section() - 1.0).abs() < f64::EPSILON);
        assert_eq!(ln.micro_time(), 1000);
    }

    // --- Type constant tests ---

    #[test]
    fn type_constants() {
        assert_eq!(TYPE_UNDEFINED, 0);
        assert_eq!(TYPE_LONGNOTE, 1);
        assert_eq!(TYPE_CHARGENOTE, 2);
        assert_eq!(TYPE_HELLCHARGENOTE, 3);
    }
}

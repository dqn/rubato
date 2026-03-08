use crate::bms_model::{BMSModel, LNTYPE_LONGNOTE};
use crate::note::{TYPE_CHARGENOTE, TYPE_HELLCHARGENOTE, TYPE_UNDEFINED};
use crate::time_line::TimeLine;

pub const TOTALNOTES_ALL: i32 = 0;
pub const TOTALNOTES_KEY: i32 = 1;
pub const TOTALNOTES_LONG_KEY: i32 = 2;
pub const TOTALNOTES_SCRATCH: i32 = 3;
pub const TOTALNOTES_LONG_SCRATCH: i32 = 4;
pub const TOTALNOTES_MINE: i32 = 5;

pub fn total_notes(model: &BMSModel) -> i32 {
    total_notes_range(model, 0, i32::MAX)
}

pub fn total_notes_with_type(model: &BMSModel, note_type: i32) -> i32 {
    total_notes_full(model, 0, i32::MAX, note_type, 0)
}

pub fn total_notes_range(model: &BMSModel, start: i32, end: i32) -> i32 {
    total_notes_full(model, start, end, TOTALNOTES_ALL, 0)
}

pub fn total_notes_range_type(model: &BMSModel, start: i32, end: i32, note_type: i32) -> i32 {
    total_notes_full(model, start, end, note_type, 0)
}

pub fn total_notes_full(model: &BMSModel, start: i32, end: i32, note_type: i32, side: i32) -> i32 {
    let mode = match model.mode() {
        Some(m) => m,
        None => return 0,
    };
    if mode.player() == 1 && side == 2 {
        return 0;
    }
    let scratch_key = mode.scratch_key();
    let mode_key = mode.key();
    let mode_player = mode.player();

    let slane_len = scratch_key.len() / (if side == 0 { 1 } else { mode_player as usize });
    let mut slane = Vec::with_capacity(slane_len);
    let start_idx = if side == 2 { slane_len } else { 0 };
    let mut i = start_idx;
    let mut index = 0;
    while index < slane_len {
        slane.push(scratch_key[i]);
        i += 1;
        index += 1;
    }

    let nlane_len = ((mode_key - scratch_key.len() as i32)
        / (if side == 0 { 1 } else { mode_player })) as usize;
    let mut nlane = Vec::with_capacity(nlane_len);
    let mut i = 0i32;
    let mut index = 0;
    while index < nlane_len {
        if !mode.is_scratch_key(i) {
            nlane.push(i);
            index += 1;
        }
        i += 1;
    }

    let lntype = model.lntype();
    let mut count = 0;
    for tl in &model.timelines {
        if tl.time() >= start && tl.time() < end {
            match note_type {
                TOTALNOTES_ALL => {
                    count += tl.total_notes_with_lntype(lntype);
                }
                TOTALNOTES_KEY => {
                    for &lane in &nlane {
                        if tl.exist_note_at(lane)
                            && let Some(note) = tl.note(lane)
                            && note.is_normal()
                        {
                            count += 1;
                        }
                    }
                }
                TOTALNOTES_LONG_KEY => {
                    for &lane in &nlane {
                        if tl.exist_note_at(lane)
                            && let Some(note) = tl.note(lane)
                            && note.is_long()
                        {
                            let ln_type = note.long_note_type();
                            if ln_type == TYPE_CHARGENOTE
                                || ln_type == TYPE_HELLCHARGENOTE
                                || (ln_type == TYPE_UNDEFINED && lntype != LNTYPE_LONGNOTE)
                                || !note.is_end()
                            {
                                count += 1;
                            }
                        }
                    }
                }
                TOTALNOTES_SCRATCH => {
                    for &lane in &slane {
                        if tl.exist_note_at(lane)
                            && let Some(note) = tl.note(lane)
                            && note.is_normal()
                        {
                            count += 1;
                        }
                    }
                }
                TOTALNOTES_LONG_SCRATCH => {
                    for &lane in &slane {
                        if let Some(note) = tl.note(lane)
                            && note.is_long()
                        {
                            let ln_type = note.long_note_type();
                            if ln_type == TYPE_CHARGENOTE
                                || ln_type == TYPE_HELLCHARGENOTE
                                || (ln_type == TYPE_UNDEFINED && lntype != LNTYPE_LONGNOTE)
                                || !note.is_end()
                            {
                                count += 1;
                            }
                        }
                    }
                }
                TOTALNOTES_MINE => {
                    for &lane in &nlane {
                        if tl.exist_note_at(lane)
                            && let Some(note) = tl.note(lane)
                            && note.is_mine()
                        {
                            count += 1;
                        }
                    }
                    for &lane in &slane {
                        if tl.exist_note_at(lane)
                            && let Some(note) = tl.note(lane)
                            && note.is_mine()
                        {
                            count += 1;
                        }
                    }
                }
                _ => {}
            }
        }
    }
    count
}

/// Java: BMSModelUtils.getAverageNotesPerTime(BMSModel, int, int)
/// Returns the average notes per 1000ms in the given time range.
pub fn average_notes_per_time(model: &BMSModel, start: i32, end: i32) -> f64 {
    if end <= start {
        return 0.0;
    }
    total_notes_range(model, start, end) as f64 * 1000.0 / (end - start) as f64
}

pub fn change_frequency(model: &mut BMSModel, freq: f32) {
    model.bpm *= freq as f64;
    for tl in &mut model.timelines {
        tl.bpm *= freq as f64;
        tl.stop = (tl.micro_stop() as f64 / (freq as f64)) as i64;
        tl.set_micro_time((tl.micro_time() as f64 / (freq as f64)) as i64);
    }
}

pub fn max_notes_per_time(model: &BMSModel, range: i32) -> f64 {
    let mut maxnotes: i32 = 0;
    let tl = &model.timelines;
    let lntype = model.lntype();
    for i in 0..tl.len() {
        let mut notes = 0;
        let mut j = i;
        while j < tl.len() && tl[j].time() < tl[i].time() + range {
            notes += tl[j].total_notes_with_lntype(lntype);
            j += 1;
        }
        maxnotes = if maxnotes < notes { notes } else { maxnotes };
    }
    maxnotes as f64
}

pub fn set_start_note_time(model: &mut BMSModel, starttime: i64) -> i64 {
    let mut margin_time: i64 = 0;
    for tl in &model.timelines {
        if tl.milli_time() >= starttime {
            break;
        }
        if tl.exist_note() {
            margin_time = starttime - tl.milli_time();
            break;
        }
    }

    if margin_time > 0 {
        let first_bpm = model.timelines[0].bpm;
        let margin_section = (margin_time as f64) * first_bpm / 240000.0;
        for tl in model.all_time_lines_mut() {
            tl.set_section(tl.get_section() + margin_section);
            tl.set_micro_time(tl.micro_time() + margin_time * 1000);
        }

        let mode_key = model.mode().map(|m| m.key()).unwrap_or(0);
        let bpm = model.bpm;

        let mut old_timelines = model.take_all_time_lines();
        let mut new_timelines: Vec<TimeLine> = Vec::with_capacity(old_timelines.len() + 1);
        let mut first = TimeLine::new(0.0, 0, mode_key);
        first.bpm = bpm;
        new_timelines.push(first);
        new_timelines.append(&mut old_timelines);
        model.timelines = new_timelines;
    }

    margin_time
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mode::Mode;
    use crate::note::Note;

    /// Helper: create a BMSModel in BEAT_7K mode with given timelines.
    fn make_model_7k(timelines: Vec<TimeLine>) -> BMSModel {
        let mut model = BMSModel::new();
        model.set_mode(Mode::BEAT_7K);
        model.bpm = 120.0;
        model.timelines = timelines;
        model
    }

    // --- total_notes ---

    #[test]
    fn total_notes_normal_notes() {
        let mut tl = TimeLine::new(0.0, 0, 8);
        tl.set_note(0, Some(Note::new_normal(1)));
        tl.set_note(1, Some(Note::new_normal(2)));
        tl.set_note(2, Some(Note::new_normal(3)));
        let model = make_model_7k(vec![tl]);
        assert_eq!(total_notes(&model), 3);
    }

    #[test]
    fn total_notes_empty_model() {
        let model = BMSModel::new();
        assert_eq!(total_notes(&model), 0);
    }

    #[test]
    fn total_notes_key_only_excludes_scratch() {
        // BEAT_7K: scratch key is lane 7, non-scratch lanes are 0-6
        let mut tl = TimeLine::new(0.0, 0, 8);
        tl.set_note(0, Some(Note::new_normal(1))); // key lane
        tl.set_note(1, Some(Note::new_normal(2))); // key lane
        tl.set_note(7, Some(Note::new_normal(3))); // scratch lane
        let model = make_model_7k(vec![tl]);

        let key_count = total_notes_with_type(&model, TOTALNOTES_KEY);
        assert_eq!(key_count, 2); // only non-scratch normal notes
    }

    #[test]
    fn total_notes_mine_only() {
        let mut tl = TimeLine::new(0.0, 0, 8);
        tl.set_note(0, Some(Note::new_normal(1)));
        tl.set_note(1, Some(Note::new_mine(2, 0.5)));
        tl.set_note(2, Some(Note::new_mine(3, 0.3)));
        let model = make_model_7k(vec![tl]);

        let mine_count = total_notes_with_type(&model, TOTALNOTES_MINE);
        assert_eq!(mine_count, 2);
    }

    #[test]
    fn total_notes_side_2_single_player_returns_zero() {
        let mut tl = TimeLine::new(0.0, 0, 8);
        tl.set_note(0, Some(Note::new_normal(1)));
        let model = make_model_7k(vec![tl]);

        // BEAT_7K is single player (player=1), so side=2 should return 0
        let count = total_notes_full(&model, 0, i32::MAX, TOTALNOTES_ALL, 2);
        assert_eq!(count, 0);
    }

    // --- average_notes_per_time ---

    #[test]
    fn average_notes_per_time_end_lte_start() {
        let model = make_model_7k(vec![]);
        assert_eq!(average_notes_per_time(&model, 100, 100), 0.0);
        assert_eq!(average_notes_per_time(&model, 100, 50), 0.0);
    }

    #[test]
    fn average_notes_per_time_basic() {
        // 10 notes in a 2000ms window => 5.0 notes per 1000ms
        let mut timelines = Vec::new();
        for i in 0..10 {
            // Spread notes across the 2000ms window (get_time returns micro_time/1000)
            let micro_time = (i as i64) * 200 * 1000; // 0, 200000, 400000, ...
            let mut tl = TimeLine::new(i as f64, micro_time, 8);
            tl.set_note(0, Some(Note::new_normal(1)));
            timelines.push(tl);
        }
        let model = make_model_7k(timelines);

        let avg = average_notes_per_time(&model, 0, 2000);
        assert!((avg - 5.0).abs() < f64::EPSILON);
    }

    // --- change_frequency ---

    #[test]
    fn change_frequency_doubles() {
        let mut tl = TimeLine::new(0.0, 1_000_000, 8);
        tl.bpm = 120.0;
        tl.stop = 500_000;
        let mut model = make_model_7k(vec![tl]);
        model.bpm = 120.0;

        change_frequency(&mut model, 2.0);

        assert!((model.bpm - 240.0).abs() < f64::EPSILON);
        let tl = &model.timelines[0];
        assert!((tl.bpm - 240.0).abs() < f64::EPSILON);
        assert_eq!(tl.micro_time(), 500_000); // halved
        assert_eq!(tl.micro_stop(), 250_000); // halved
    }

    #[test]
    fn change_frequency_halves() {
        let mut tl = TimeLine::new(0.0, 1_000_000, 8);
        tl.bpm = 120.0;
        tl.stop = 500_000;
        let mut model = make_model_7k(vec![tl]);
        model.bpm = 120.0;

        change_frequency(&mut model, 0.5);

        assert!((model.bpm - 60.0).abs() < f64::EPSILON);
        let tl = &model.timelines[0];
        assert!((tl.bpm - 60.0).abs() < f64::EPSILON);
        assert_eq!(tl.micro_time(), 2_000_000); // doubled
        assert_eq!(tl.micro_stop(), 1_000_000); // doubled
    }

    // --- max_notes_per_time ---

    #[test]
    fn max_notes_per_time_clustered_notes() {
        // 3 notes at the same time (time=0) within range=1000
        let mut tl = TimeLine::new(0.0, 0, 8);
        tl.set_note(0, Some(Note::new_normal(1)));
        tl.set_note(1, Some(Note::new_normal(2)));
        tl.set_note(2, Some(Note::new_normal(3)));
        let model = make_model_7k(vec![tl]);

        assert!((max_notes_per_time(&model, 1000) - 3.0).abs() < f64::EPSILON);
    }

    #[test]
    fn max_notes_per_time_empty_model() {
        let model = make_model_7k(vec![]);
        assert!((max_notes_per_time(&model, 1000)).abs() < f64::EPSILON);
    }

    // --- set_start_note_time ---

    #[test]
    fn set_start_note_time_note_before_starttime_inserts_padding() {
        // First note is at time=0ms, starttime=1000ms
        let mut tl = TimeLine::new(0.0, 0, 8);
        tl.bpm = 120.0;
        tl.set_note(0, Some(Note::new_normal(1)));
        let mut model = make_model_7k(vec![tl]);
        model.bpm = 120.0;

        let margin = set_start_note_time(&mut model, 1000);
        assert_eq!(margin, 1000);

        // Should have inserted a padding timeline at the beginning
        assert_eq!(model.timelines.len(), 2);
        // First timeline is the padding (time=0, section=0)
        assert_eq!(model.timelines[0].micro_time(), 0);
    }

    #[test]
    fn set_start_note_time_note_after_starttime_returns_zero() {
        // First note at 2000ms, starttime=1000ms
        let mut tl = TimeLine::new(0.0, 2_000_000, 8);
        tl.bpm = 120.0;
        tl.set_note(0, Some(Note::new_normal(1)));
        let mut model = make_model_7k(vec![tl]);

        let margin = set_start_note_time(&mut model, 1000);
        assert_eq!(margin, 0);
        // No padding inserted
        assert_eq!(model.timelines.len(), 1);
    }

    #[test]
    fn set_start_note_time_no_notes_returns_zero() {
        let mut tl = TimeLine::new(0.0, 0, 8);
        tl.bpm = 120.0;
        // No notes set
        let mut model = make_model_7k(vec![tl]);

        let margin = set_start_note_time(&mut model, 1000);
        assert_eq!(margin, 0);
    }

    // --- constant values ---

    #[test]
    fn totalnotes_constants() {
        assert_eq!(TOTALNOTES_ALL, 0);
        assert_eq!(TOTALNOTES_KEY, 1);
        assert_eq!(TOTALNOTES_LONG_KEY, 2);
        assert_eq!(TOTALNOTES_SCRATCH, 3);
        assert_eq!(TOTALNOTES_LONG_SCRATCH, 4);
        assert_eq!(TOTALNOTES_MINE, 5);
    }

    #[test]
    fn total_notes_scratch_only() {
        // BEAT_7K: scratch key is lane 7
        let mut tl = TimeLine::new(0.0, 0, 8);
        tl.set_note(0, Some(Note::new_normal(1))); // key lane
        tl.set_note(7, Some(Note::new_normal(2))); // scratch lane
        let model = make_model_7k(vec![tl]);

        let scratch_count = total_notes_with_type(&model, TOTALNOTES_SCRATCH);
        assert_eq!(scratch_count, 1);
    }
}

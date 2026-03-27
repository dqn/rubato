use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::{Arc, Condvar, Mutex};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use bms::model::bms_model::BMSModel;
use bms::model::note::Note;
use rubato_types::sync_utils::lock_or_recover;

/// A BG note that needs to be played by the audio driver.
///
/// The BG autoplay thread produces these; the caller drains them via
/// `KeySoundProcessor::drain_pending_bg_notes()` and forwards each to
/// `AudioDriver::play_note(&note, volume, 0)`.
#[derive(Clone, Debug)]
pub struct BgNoteCommand {
    pub note: Note,
    pub volume: f32,
}

/// Shared state between the BG autoplay thread and the main thread.
struct BgShared {
    /// Current play time in microseconds (`now_micro_time_for_id(TIMER_PLAY)`).
    /// Updated by the main thread each frame.
    play_time: AtomicI64,
    /// Stop flag + condvar for interruptible sleep.
    /// `stop_bg_play()` sets the flag and notifies the condvar so the thread
    /// wakes immediately instead of sleeping until the next timeline.
    stop_flag: Mutex<bool>,
    stop_signal: Condvar,
    /// Current effective volume for BG notes.
    /// Stored as raw f32 bits via `f32::to_bits()` / `f32::from_bits()`.
    volume_bits: AtomicI64,
    /// Queue of notes to play, drained by the main thread.
    pending_notes: Mutex<Vec<BgNoteCommand>>,
}

/// Pre-extracted BG timeline data sent to the thread.
/// Each entry holds the micro time and the BG notes at that timeline.
struct BgTimelineEntry {
    micro_time: i64,
    notes: Vec<Note>,
}

/// BG lane autoplay thread handle.
struct AutoplayHandle {
    shared: Arc<BgShared>,
    thread: Option<JoinHandle<()>>,
}

/// Key sound processor for BG lane playback.
///
/// Translated from: Java KeySoundProcessor + AutoplayThread inner class.
///
/// # Architecture
///
/// In Java, `AutoplayThread` directly accesses `player.timer` and `audio`.
/// In Rust, we decouple the thread from `BMSPlayer` and `AudioDriver`:
///
/// 1. The BG thread reads the current play time via `AtomicI64` (updated by the
///    main thread each frame).
/// 2. When a note should play, the thread pushes a `BgNoteCommand` into a shared
///    queue.
/// 3. The main thread drains the queue via `drain_pending_bg_notes()` and calls
///    `AudioDriver::play_note()` for each command.
pub struct KeySoundProcessor {
    handle: Option<AutoplayHandle>,
}

impl Default for KeySoundProcessor {
    fn default() -> Self {
        Self::new()
    }
}

impl KeySoundProcessor {
    pub fn new() -> Self {
        KeySoundProcessor { handle: None }
    }

    /// Start BG lane autoplay.
    ///
    /// Spawns a background thread that tracks timeline positions and enqueues
    /// note-play commands at the correct timing.
    ///
    /// Translated from: Java `KeySoundProcessor.startBGPlay(BMSModel, long)`.
    ///
    /// # Arguments
    /// * `model` - The BMS model containing BG note timelines.
    /// * `starttime` - Start time offset in microseconds.
    /// * `bg_volume` - Initial BG volume from `AudioConfig.bgvolume`.
    pub fn start_bg_play(&mut self, model: &BMSModel, starttime: i64, bg_volume: f32) {
        // Stop any existing thread first
        self.stop_bg_play();

        // Pre-extract BG timeline data (notes + times) so the thread owns them.
        let mut entries: Vec<BgTimelineEntry> = Vec::new();
        for tl in &model.timelines {
            let bg_notes = tl.back_ground_notes();
            if !bg_notes.is_empty() {
                entries.push(BgTimelineEntry {
                    micro_time: tl.micro_time(),
                    notes: bg_notes.to_vec(),
                });
            }
        }

        if entries.is_empty() {
            return;
        }

        let shared = Arc::new(BgShared {
            play_time: AtomicI64::new(0),
            stop_flag: Mutex::new(false),
            stop_signal: Condvar::new(),
            volume_bits: AtomicI64::new((bg_volume).to_bits() as i64),
            pending_notes: Mutex::new(Vec::new()),
        });

        let shared_clone = Arc::clone(&shared);
        let thread = thread::Builder::new()
            .name("bg-autoplay".into())
            .spawn(move || {
                autoplay_run(shared_clone, entries, starttime);
            })
            .ok();

        if let Some(thread) = thread {
            self.handle = Some(AutoplayHandle {
                shared,
                thread: Some(thread),
            });
        }
    }

    /// Stop BG lane autoplay.
    ///
    /// Translated from: Java `KeySoundProcessor.stopBGPlay()`.
    pub fn stop_bg_play(&mut self) {
        if let Some(ref mut handle) = self.handle {
            {
                let mut guard = lock_or_recover(&handle.shared.stop_flag);
                *guard = true;
            }
            handle.shared.stop_signal.notify_one();
            if let Some(thread) = handle.thread.take() {
                let _ = thread.join();
            }
        }
        self.handle = None;
    }

    /// Update the current play time for the BG autoplay thread.
    ///
    /// Should be called each frame from the main render loop with the value of
    /// `timer.now_micro_time_for_id(TIMER_PLAY)`.
    pub fn update_play_time(&self, time: i64) {
        if let Some(ref handle) = self.handle {
            handle.shared.play_time.store(time, Ordering::Release);
        }
    }

    /// Update the effective BG volume for the autoplay thread.
    ///
    /// In Java: `player.getAdjustedVolume()` is checked first; if negative,
    /// `config.getAudioConfig().getBgvolume()` is used instead.
    /// The caller should pass the resolved volume.
    pub fn update_volume(&self, volume: f32) {
        if let Some(ref handle) = self.handle {
            handle
                .shared
                .volume_bits
                .store(volume.to_bits() as i64, Ordering::Release);
        }
    }

    /// Drain all pending BG note commands.
    ///
    /// Returns the notes that the BG autoplay thread has determined should play.
    /// The caller should call `AudioDriver::play_note(note, volume, 0)` for each.
    pub fn drain_pending_bg_notes(&self) -> Vec<BgNoteCommand> {
        if let Some(ref handle) = self.handle {
            let mut pending = lock_or_recover(&handle.shared.pending_notes);
            std::mem::take(&mut *pending)
        } else {
            Vec::new()
        }
    }

    /// Returns true if the BG autoplay thread is currently running.
    pub fn is_bg_playing(&self) -> bool {
        self.handle.is_some()
    }
}

/// BG autoplay thread main loop.
///
/// Translated from: Java `AutoplayThread.run()`.
///
/// 1. Find starting position from starttime.
/// 2. Loop while !stop.
/// 3. Get current micro time from the shared play_time atomic.
/// 4. Get volume from the shared volume_bits atomic.
/// 5. Enqueue all BG notes in timelines up to current time.
/// 6. Sleep until next timeline.
/// 7. Break when past last timeline time.
fn autoplay_run(shared: Arc<BgShared>, entries: Vec<BgTimelineEntry>, starttime: i64) {
    let lasttime = entries.last().map_or(0, |e| e.micro_time);

    // Find starting position: skip timelines before starttime.
    // Translated from: Java `for (long time = starttime; p < timelines.length && timelines[p].getMicroTime() < time; p++);`
    let mut p: usize = 0;
    while p < entries.len() && entries[p].micro_time < starttime {
        p += 1;
    }

    loop {
        if *lock_or_recover(&shared.stop_flag) {
            break;
        }

        let time = shared.play_time.load(Ordering::Acquire);
        let volume_bits = shared.volume_bits.load(Ordering::Acquire) as u32;
        let volume = f32::from_bits(volume_bits);

        // Play all BG notes in timelines up to current time.
        while p < entries.len() && entries[p].micro_time <= time {
            let cmds: Vec<BgNoteCommand> = entries[p]
                .notes
                .iter()
                .map(|n| BgNoteCommand {
                    note: n.clone(),
                    volume,
                })
                .collect();

            {
                let mut pending = lock_or_recover(&shared.pending_notes);
                pending.extend(cmds);
            }
            p += 1;
        }

        // Interruptible sleep until next timeline.
        // Uses Condvar so stop_bg_play() can wake us immediately.
        if p < entries.len() {
            let sleeptime = entries[p].micro_time - time;
            if sleeptime > 0 {
                // Java: Thread.sleep(sleeptime / 1000) — converts micros to millis.
                let guard = lock_or_recover(&shared.stop_flag);
                if !*guard {
                    let _ = shared
                        .stop_signal
                        .wait_timeout(guard, Duration::from_micros(sleeptime as u64));
                }
            }
        }

        // Break when past last timeline time.
        if time >= lasttime {
            break;
        }
    }
}

impl Drop for KeySoundProcessor {
    fn drop(&mut self) {
        self.stop_bg_play();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bms::model::time_line::TimeLine;

    fn make_model_with_bg_notes(times: &[(i64, Vec<i32>)]) -> BMSModel {
        let mut model = BMSModel::new();
        let mut timelines = Vec::new();
        for (micro_time, wavs) in times {
            let mut tl = TimeLine::new(0.0, *micro_time, 8);
            for wav in wavs {
                tl.add_back_ground_note(Note::new_normal(*wav));
            }
            timelines.push(tl);
        }
        model.timelines = timelines;
        model
    }

    #[test]
    fn new_creates_default_state() {
        let ksp = KeySoundProcessor::new();
        assert!(!ksp.is_bg_playing());
        assert!(ksp.drain_pending_bg_notes().is_empty());
    }

    #[test]
    fn start_bg_play_with_empty_model_does_not_spawn_thread() {
        let mut ksp = KeySoundProcessor::new();
        let model = BMSModel::new();
        ksp.start_bg_play(&model, 0, 0.5);
        assert!(!ksp.is_bg_playing());
    }

    #[test]
    fn start_bg_play_with_no_bg_notes_does_not_spawn_thread() {
        let mut ksp = KeySoundProcessor::new();
        let mut model = BMSModel::new();
        // Add a timeline with no BG notes.
        model.timelines = vec![TimeLine::new(0.0, 1000, 8)];
        ksp.start_bg_play(&model, 0, 0.5);
        assert!(!ksp.is_bg_playing());
    }

    #[test]
    fn start_and_stop_bg_play() {
        let mut ksp = KeySoundProcessor::new();
        let model = make_model_with_bg_notes(&[(1_000_000, vec![1])]);
        ksp.start_bg_play(&model, 0, 0.5);
        assert!(ksp.is_bg_playing());
        ksp.stop_bg_play();
        assert!(!ksp.is_bg_playing());
    }

    #[test]
    fn bg_notes_enqueued_when_time_advances() {
        let mut ksp = KeySoundProcessor::new();
        // BG notes at 0us and 100us.
        let model = make_model_with_bg_notes(&[(0, vec![10]), (100, vec![20, 21])]);
        ksp.start_bg_play(&model, 0, 0.8);
        assert!(ksp.is_bg_playing());

        // Set play time past both timelines.
        ksp.update_play_time(200);

        // Give the thread time to process.
        thread::sleep(Duration::from_millis(50));

        let notes = ksp.drain_pending_bg_notes();
        // Should have 3 notes total (1 from first timeline, 2 from second).
        assert_eq!(notes.len(), 3);
        assert_eq!(notes[0].note.wav(), 10);
        assert!((notes[0].volume - 0.8).abs() < f32::EPSILON);
        assert_eq!(notes[1].note.wav(), 20);
        assert_eq!(notes[2].note.wav(), 21);

        ksp.stop_bg_play();
    }

    #[test]
    fn bg_play_skips_past_timelines_based_on_starttime() {
        let mut ksp = KeySoundProcessor::new();
        // BG notes at 100us, 200us, 300us.
        let model = make_model_with_bg_notes(&[(100, vec![1]), (200, vec![2]), (300, vec![3])]);
        // Start at 200us — should skip the first timeline.
        ksp.start_bg_play(&model, 200, 0.5);

        // Set play time to 300us.
        ksp.update_play_time(300);
        thread::sleep(Duration::from_millis(50));

        let notes = ksp.drain_pending_bg_notes();
        // Should have notes from timelines at 200us and 300us (wavs 2 and 3),
        // but NOT from 100us (wav 1).
        let wavs: Vec<i32> = notes.iter().map(|n| n.note.wav()).collect();
        assert!(
            !wavs.contains(&1),
            "should not contain wav 1 (before starttime)"
        );
        assert!(wavs.contains(&2), "should contain wav 2");
        assert!(wavs.contains(&3), "should contain wav 3");

        ksp.stop_bg_play();
    }

    #[test]
    fn update_volume_reflected_in_commands() {
        let mut ksp = KeySoundProcessor::new();
        // Use two timelines close together so the thread's sleep between them is short.
        let model = make_model_with_bg_notes(&[(100, vec![1]), (200, vec![2])]);
        ksp.start_bg_play(&model, 0, 0.3);

        // Play first note at volume 0.3.
        ksp.update_play_time(100);
        thread::sleep(Duration::from_millis(50));
        let notes1 = ksp.drain_pending_bg_notes();
        assert!(!notes1.is_empty());
        assert!((notes1[0].volume - 0.3).abs() < f32::EPSILON);

        // Update volume and play second note.
        ksp.update_volume(0.9);
        ksp.update_play_time(200);
        thread::sleep(Duration::from_millis(50));
        let notes2 = ksp.drain_pending_bg_notes();
        assert!(!notes2.is_empty());
        assert!((notes2[0].volume - 0.9).abs() < f32::EPSILON);

        ksp.stop_bg_play();
    }

    #[test]
    fn thread_exits_after_last_timeline() {
        let mut ksp = KeySoundProcessor::new();
        let model = make_model_with_bg_notes(&[(0, vec![1])]);
        ksp.start_bg_play(&model, 0, 0.5);

        // Set time past the last timeline.
        ksp.update_play_time(100);
        // Give the thread time to exit naturally.
        thread::sleep(Duration::from_millis(50));

        // Drain to verify it produced the note.
        let notes = ksp.drain_pending_bg_notes();
        assert_eq!(notes.len(), 1);
        assert_eq!(notes[0].note.wav(), 1);

        ksp.stop_bg_play();
    }

    #[test]
    fn stop_terminates_thread_early() {
        let mut ksp = KeySoundProcessor::new();
        // Far-future timeline that the thread would sleep for.
        let model = make_model_with_bg_notes(&[(10_000_000_000, vec![1])]);
        ksp.start_bg_play(&model, 0, 0.5);
        assert!(ksp.is_bg_playing());

        // Stop should return promptly even though the thread is sleeping.
        // The thread checks `stop` after waking from sleep.
        // We can't guarantee instant termination, but stop_bg_play() joins the thread.
        ksp.stop_bg_play();
        assert!(!ksp.is_bg_playing());
    }

    #[test]
    fn drain_returns_empty_when_no_thread() {
        let ksp = KeySoundProcessor::new();
        assert!(ksp.drain_pending_bg_notes().is_empty());
    }

    #[test]
    fn drop_stops_thread() {
        let model = make_model_with_bg_notes(&[(10_000_000_000, vec![1])]);
        let mut ksp = KeySoundProcessor::new();
        ksp.start_bg_play(&model, 0, 0.5);
        assert!(ksp.is_bg_playing());
        // Drop should join the thread.
        drop(ksp);
        // If we get here without hanging, the test passes.
    }

    #[test]
    fn restart_bg_play_stops_previous_thread() {
        let mut ksp = KeySoundProcessor::new();
        let model1 = make_model_with_bg_notes(&[(0, vec![1])]);
        ksp.start_bg_play(&model1, 0, 0.5);
        assert!(ksp.is_bg_playing());

        // Starting again should stop the first thread.
        let model2 = make_model_with_bg_notes(&[(0, vec![2])]);
        ksp.start_bg_play(&model2, 0, 0.5);
        assert!(ksp.is_bg_playing());

        ksp.update_play_time(100);
        thread::sleep(Duration::from_millis(50));
        let notes = ksp.drain_pending_bg_notes();
        // Should only have notes from the second model.
        assert!(!notes.is_empty());
        assert_eq!(notes[0].note.wav(), 2);

        ksp.stop_bg_play();
    }
}

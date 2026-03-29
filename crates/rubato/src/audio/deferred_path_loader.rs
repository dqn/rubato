/// Deferred path sound loader - loads audio files on a background thread
/// to avoid blocking the render thread on cache misses.
///
/// Used by GdxSoundDriver, GdxAudioDeviceDriver, and PortAudioDriver.
use std::collections::{HashMap, HashSet};
use std::sync::mpsc;
use std::thread::JoinHandle;

use kira::sound::static_sound::StaticSoundData;

/// Result of a background path sound load.
struct PathLoadResult {
    path: String,
    sound: Option<StaticSoundData>,
}

/// A loaded sound with its path and pending play requests (volume, loop).
type LoadedSoundEntry = (String, StaticSoundData, Vec<(f32, bool)>);

/// Manages deferred (non-blocking) loading of path-based audio files.
///
/// On cache miss, spawns a background thread to load the file. The loaded
/// sound data is received via channel on the next `poll()` call and cached
/// for immediate playback.
pub(crate) struct DeferredPathLoader {
    tx: mpsc::Sender<PathLoadResult>,
    rx: mpsc::Receiver<PathLoadResult>,
    /// Paths currently being loaded in background threads.
    loading: HashSet<String>,
    /// Play requests waiting for their path to finish loading.
    pending_plays: Vec<(String, f32, bool)>,
    /// JoinHandles for background load threads, keyed by path.
    /// Used to detect and log panics from loader threads.
    loading_handles: HashMap<String, JoinHandle<()>>,
}

impl DeferredPathLoader {
    pub fn new() -> Self {
        let (tx, rx) = mpsc::channel();
        Self {
            tx,
            rx,
            loading: HashSet::new(),
            pending_plays: Vec::new(),
            loading_handles: HashMap::new(),
        }
    }

    /// Maximum number of concurrent background load threads.
    const MAX_CONCURRENT_LOADS: usize = 8;

    /// Queue a background load for the given path if not already loading.
    /// Also records the play request so it can be fulfilled when loading completes.
    ///
    /// When the maximum number of concurrent loads is reached and the path is
    /// not already loading, both the load and the play request are skipped.
    /// During rapid song scrolling the user has already moved past the track,
    /// so the sound would never be heard anyway. This prevents unbounded growth
    /// of `pending_plays` with entries that no background thread will ever drain.
    pub fn request_load(&mut self, path: &str, volume: f32, loop_play: bool) {
        if self.loading.contains(path) {
            // A thread is already loading this path; just record the play request
            // so it will be fulfilled when the load completes.
            self.pending_plays
                .push((path.to_string(), volume, loop_play));
            return;
        }

        if self.loading.len() >= Self::MAX_CONCURRENT_LOADS {
            // Concurrency limit reached and no thread is loading this path.
            // Skip both the load and the play request to avoid orphaned
            // pending_plays entries that would never be drained.
            return;
        }

        self.loading.insert(path.to_string());
        let tx = self.tx.clone();
        let path_owned = path.to_string();
        let path_key = path.to_string();
        match std::thread::Builder::new()
            .name(format!("path-audio-load:{}", path))
            .spawn(move || {
                let candidates = crate::audio::audio_driver::paths(&path_owned);
                let mut loaded = None;
                for candidate in &candidates {
                    if let Ok(sound_data) = StaticSoundData::from_file(candidate) {
                        loaded = Some(sound_data);
                        break;
                    }
                }
                let _ = tx.send(PathLoadResult {
                    path: path_owned,
                    sound: loaded,
                });
            }) {
            Ok(handle) => {
                self.loading_handles.insert(path_key, handle);
                self.pending_plays
                    .push((path.to_string(), volume, loop_play));
            }
            Err(e) => {
                log::warn!("Failed to spawn audio load thread for {}: {}", path, e);
                self.loading.remove(path);
                // Don't add to pending_plays -- no thread will drain it.
            }
        }
    }

    /// Queue a background preload for the given path without recording a play request.
    /// The loaded sound will be inserted into the cache on the next `poll()` call
    /// with an empty plays list, so the caller just caches it without playing.
    pub fn request_preload(&mut self, path: &str) {
        if self.loading.contains(path) {
            // Already being loaded; no need to duplicate.
            return;
        }

        if self.loading.len() >= Self::MAX_CONCURRENT_LOADS {
            return;
        }

        self.loading.insert(path.to_string());
        let tx = self.tx.clone();
        let path_owned = path.to_string();
        let path_key = path.to_string();
        match std::thread::Builder::new()
            .name(format!("path-audio-preload:{}", path))
            .spawn(move || {
                let candidates = crate::audio::audio_driver::paths(&path_owned);
                let mut loaded = None;
                for candidate in &candidates {
                    if let Ok(sound_data) = StaticSoundData::from_file(candidate) {
                        loaded = Some(sound_data);
                        break;
                    }
                }
                let _ = tx.send(PathLoadResult {
                    path: path_owned,
                    sound: loaded,
                });
            }) {
            Ok(handle) => {
                self.loading_handles.insert(path_key, handle);
                // No pending play recorded -- preload only.
            }
            Err(e) => {
                log::warn!("Failed to spawn audio preload thread for {}: {}", path, e);
                self.loading.remove(path);
            }
        }
    }

    /// Poll for completed background loads. Returns newly loaded sounds
    /// and their pending play requests.
    ///
    /// Caller is responsible for inserting into `path_sound_cache` and playing.
    pub fn poll(&mut self) -> Vec<LoadedSoundEntry> {
        let mut results = Vec::new();

        while let Ok(result) = self.rx.try_recv() {
            self.loading.remove(&result.path);
            // Join the handle for this completed path to detect panics.
            if let Some(handle) = self.loading_handles.remove(&result.path)
                && let Err(e) = handle.join()
            {
                log::warn!("Audio load thread panicked for {}: {:?}", result.path, e);
            }
            if let Some(sound) = result.sound {
                // Collect all pending plays for this path
                let plays: Vec<(f32, bool)> = self
                    .pending_plays
                    .iter()
                    .filter(|(p, _, _)| *p == result.path)
                    .map(|(_, v, l)| (*v, *l))
                    .collect();
                self.pending_plays.retain(|(p, _, _)| *p != result.path);
                results.push((result.path, sound, plays));
            } else {
                // Load failed - discard pending plays for this path
                self.pending_plays.retain(|(p, _, _)| *p != result.path);
            }
        }

        // Join finished handles whose results were never received (thread
        // panicked before sending). This prevents silent panic swallowing.
        let finished_paths: Vec<String> = self
            .loading_handles
            .iter()
            .filter(|(_, h)| h.is_finished())
            .map(|(p, _)| p.clone())
            .collect();
        for path in finished_paths {
            if let Some(handle) = self.loading_handles.remove(&path) {
                if let Err(e) = handle.join() {
                    log::warn!("Audio load thread panicked for {}: {:?}", path, e);
                }
                // Clean up associated state since the thread will never send a result.
                self.loading.remove(&path);
                self.pending_plays.retain(|(p, _, _)| *p != path);
            }
        }

        results
    }

    /// Returns true if the given path has a pending play request (i.e., a
    /// background load is in progress or completed but not yet polled, and a
    /// play was requested for it). This allows callers like `is_playing_path()`
    /// to treat deferred loads as "playing" so the preview processor doesn't
    /// mistakenly think playback finished.
    pub fn has_pending_play(&self, path: &str) -> bool {
        self.pending_plays.iter().any(|(p, _, _)| p == path)
    }

    /// Remove all pending play requests for the given path. The background
    /// load (if still in progress) is NOT cancelled -- it will complete and
    /// be cached for future use, but no playback will be triggered.
    pub fn cancel_pending_plays(&mut self, path: &str) {
        self.pending_plays.retain(|(p, _, _)| p != path);
    }

    /// Drain all pending state (e.g., on dispose).
    ///
    /// Joins already-finished handles to log panics; drops in-flight handles
    /// to avoid blocking dispose (per "Loader thread: drop handle, don't join").
    pub fn clear(&mut self) {
        self.loading.clear();
        self.pending_plays.clear();
        // Drain any remaining messages
        while self.rx.try_recv().is_ok() {}
        // Join finished handles to observe panics, drop the rest.
        for (path, handle) in self.loading_handles.drain() {
            if handle.is_finished()
                && let Err(e) = handle.join()
            {
                log::warn!("Audio load thread panicked for {}: {:?}", path, e);
            }
            // In-flight handles are dropped (detached), which is safe --
            // they will complete in the background and their send() will
            // fail harmlessly since the receiver is about to be dropped.
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Regression: when the concurrency limit is reached and a new (not-already-loading)
    /// path is requested, pending_plays must NOT grow. Without the fix, orphaned entries
    /// accumulated because no background thread would ever produce a result to drain them.
    #[test]
    fn pending_plays_not_added_when_concurrency_limit_reached() {
        let mut loader = DeferredPathLoader::new();

        // Simulate MAX_CONCURRENT_LOADS paths already in flight by inserting
        // directly into `loading` (avoids spawning real threads).
        for i in 0..DeferredPathLoader::MAX_CONCURRENT_LOADS {
            loader.loading.insert(format!("already_loading_{i}"));
        }
        assert_eq!(
            loader.loading.len(),
            DeferredPathLoader::MAX_CONCURRENT_LOADS
        );

        // Request a load for a path that is NOT in the loading set.
        // The concurrency limit is full, so this should be silently dropped.
        loader.request_load("new_path_over_limit", 1.0, false);

        assert!(
            loader.pending_plays.is_empty(),
            "pending_plays should remain empty when concurrency limit is reached \
             and the path is not already loading, but had {} entries",
            loader.pending_plays.len()
        );
        // The path should not have been added to loading either.
        assert!(
            !loader.loading.contains("new_path_over_limit"),
            "path should not be added to loading set when limit is reached"
        );
    }

    /// When a path IS already loading, additional request_load calls for the same
    /// path should still add to pending_plays (the thread will eventually drain them).
    #[test]
    fn pending_plays_added_when_path_already_loading() {
        let mut loader = DeferredPathLoader::new();

        // Simulate MAX_CONCURRENT_LOADS paths in flight, one of which is "my_path".
        loader.loading.insert("my_path".to_string());
        for i in 1..DeferredPathLoader::MAX_CONCURRENT_LOADS {
            loader.loading.insert(format!("other_{i}"));
        }

        // Even though the limit is reached, "my_path" IS already loading,
        // so the play request should be recorded.
        loader.request_load("my_path", 0.8, true);

        assert_eq!(
            loader.pending_plays.len(),
            1,
            "pending_plays should have exactly 1 entry for an already-loading path"
        );
        assert_eq!(loader.pending_plays[0].0, "my_path");
        assert!((loader.pending_plays[0].1 - 0.8).abs() < f32::EPSILON);
        assert!(loader.pending_plays[0].2);
    }

    /// When a background thread panics before sending its result, poll()
    /// must detect the finished-but-panicked handle, clean up loading/pending
    /// state, and not leave orphaned entries.
    #[test]
    fn poll_cleans_up_panicked_thread_handle() {
        let mut loader = DeferredPathLoader::new();

        // Spawn a thread that panics without sending a PathLoadResult.
        let handle = std::thread::Builder::new()
            .name("test-panic-thread".into())
            .spawn(|| {
                panic!("simulated audio load panic");
            })
            .unwrap();

        // Wait for the thread to finish (it will panic).
        while !handle.is_finished() {
            std::thread::sleep(std::time::Duration::from_millis(10));
        }

        // Manually insert the panicked handle and associated state.
        let path = "panicked_path".to_string();
        loader.loading.insert(path.clone());
        loader.pending_plays.push((path.clone(), 1.0, false));
        loader.loading_handles.insert(path.clone(), handle);

        // poll() should detect the finished handle, join it (observing the
        // panic), and clean up loading + pending_plays for that path.
        let results = loader.poll();

        assert!(results.is_empty(), "no sound data from a panicked thread");
        assert!(
            !loader.loading.contains("panicked_path"),
            "loading set should be cleaned up after panic"
        );
        assert!(
            loader.pending_plays.is_empty(),
            "pending_plays should be cleaned up after panic"
        );
        assert!(
            loader.loading_handles.is_empty(),
            "loading_handles should be cleaned up after panic"
        );
    }

    /// clear() joins finished handles and drops in-flight ones without blocking.
    #[test]
    fn clear_joins_finished_handles_and_drops_inflight() {
        let mut loader = DeferredPathLoader::new();

        // Spawn a normal thread that completes quickly.
        let handle = std::thread::Builder::new()
            .name("test-normal-thread".into())
            .spawn(|| {
                // no-op, completes immediately
            })
            .unwrap();

        // Wait for the thread to finish.
        while !handle.is_finished() {
            std::thread::sleep(std::time::Duration::from_millis(10));
        }

        loader
            .loading_handles
            .insert("done_path".to_string(), handle);
        loader.loading.insert("done_path".to_string());

        loader.clear();

        assert!(loader.loading.is_empty());
        assert!(loader.pending_plays.is_empty());
        assert!(loader.loading_handles.is_empty());
    }

    /// request_preload() should add to loading set but NOT add to pending_plays.
    #[test]
    fn request_preload_does_not_record_play_request() {
        let mut loader = DeferredPathLoader::new();

        // Simulate a path already in the loading set (avoid spawning real thread).
        // Use request_preload for a path that's already loading -- it should
        // return early without adding a play request.
        loader.loading.insert("preload_path".to_string());

        loader.request_preload("preload_path");

        assert!(
            loader.pending_plays.is_empty(),
            "request_preload should never add to pending_plays, but had {} entries",
            loader.pending_plays.len()
        );
    }

    /// request_preload respects concurrency limit.
    #[test]
    fn request_preload_skips_when_concurrency_limit_reached() {
        let mut loader = DeferredPathLoader::new();

        for i in 0..DeferredPathLoader::MAX_CONCURRENT_LOADS {
            loader.loading.insert(format!("loading_{i}"));
        }

        loader.request_preload("new_preload");

        assert!(
            !loader.loading.contains("new_preload"),
            "preload should not be added when concurrency limit is reached"
        );
        assert!(loader.pending_plays.is_empty());
    }

    /// Contrast: request_load for an already-loading path DOES add a play request,
    /// but request_preload for the same path does NOT.
    #[test]
    fn request_preload_vs_request_load_play_recording() {
        let mut loader = DeferredPathLoader::new();
        loader.loading.insert("shared_path".to_string());

        // request_load adds a play request
        loader.request_load("shared_path", 0.5, false);
        assert_eq!(loader.pending_plays.len(), 1);

        // request_preload does not
        loader.request_preload("shared_path");
        assert_eq!(
            loader.pending_plays.len(),
            1,
            "request_preload should not add another pending play"
        );
    }

    #[test]
    fn has_pending_play_returns_true_for_pending_path() {
        let mut loader = DeferredPathLoader::new();
        loader.loading.insert("my_path".to_string());
        loader
            .pending_plays
            .push(("my_path".to_string(), 0.5, false));

        assert!(
            loader.has_pending_play("my_path"),
            "should return true when pending_plays has an entry for the path"
        );
    }

    #[test]
    fn has_pending_play_returns_false_for_preload_only() {
        let mut loader = DeferredPathLoader::new();
        loader.loading.insert("preload_path".to_string());
        // No pending_plays entry (preload only)

        assert!(
            !loader.has_pending_play("preload_path"),
            "should return false when path is loading but has no pending play"
        );
    }

    #[test]
    fn has_pending_play_returns_false_for_unknown_path() {
        let loader = DeferredPathLoader::new();

        assert!(
            !loader.has_pending_play("unknown"),
            "should return false for unknown path"
        );
    }

    #[test]
    fn cancel_pending_plays_removes_entries() {
        let mut loader = DeferredPathLoader::new();
        loader
            .pending_plays
            .push(("path_a".to_string(), 0.5, false));
        loader.pending_plays.push(("path_b".to_string(), 0.8, true));
        loader
            .pending_plays
            .push(("path_a".to_string(), 0.6, false));

        loader.cancel_pending_plays("path_a");

        assert_eq!(loader.pending_plays.len(), 1);
        assert_eq!(loader.pending_plays[0].0, "path_b");
    }

    #[test]
    fn cancel_pending_plays_does_not_affect_loading_set() {
        let mut loader = DeferredPathLoader::new();
        loader.loading.insert("my_path".to_string());
        loader
            .pending_plays
            .push(("my_path".to_string(), 0.5, false));

        loader.cancel_pending_plays("my_path");

        assert!(
            loader.loading.contains("my_path"),
            "loading set should not be affected by cancel_pending_plays"
        );
        assert!(loader.pending_plays.is_empty());
    }
}
